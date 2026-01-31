"""CLI entry point for arkai-inbox.

Main commands:
- pipeline: Run fixture-based pipeline testing
- gmail: Run pipeline on live Gmail messages
- (future) triage: Interactive email triage loop
"""

import json
from pathlib import Path
from typing import Optional

import typer
from rich.console import Console
from rich.panel import Panel
from rich.table import Table

from arkai_inbox.audit import AuditLog, log_pre_gate
from arkai_inbox.ingestion.gmail import parse_gmail_message
from arkai_inbox.ingestion.gmail_client import GmailClient
from arkai_inbox.models import create_evidence_bundle, CriticEvidenceBundle, EmailRecord

app = typer.Typer(
    name="arkai-inbox",
    help="Unified Inbox Review System",
    no_args_is_help=True,
)
console = Console()


def _tier_color(tier: str) -> str:
    """Return Rich color for tier."""
    colors = {
        "PASS": "green",
        "REVIEW": "yellow",
        "QUARANTINE": "red",
    }
    return colors.get(tier, "white")


def _truncate(text: str, max_len: int = 200) -> str:
    """Truncate text with ellipsis."""
    if not text:
        return ""
    if len(text) <= max_len:
        return text
    return text[:max_len] + "..."


def _format_evidence_panel(filename: str, bundle: CriticEvidenceBundle) -> Panel:
    """Format a CriticEvidenceBundle as a Rich panel."""
    tier_color = _tier_color(bundle.quarantine_tier)

    # Build content lines
    lines = [
        f"[bold]From:[/bold] {bundle.sender}",
        f"[bold]Subject:[/bold] {bundle.subject or '(no subject)'}",
        f"[bold]Tier:[/bold] [{tier_color}]{bundle.quarantine_tier}[/{tier_color}]",
        f"[bold]Reasons:[/bold] {bundle.quarantine_reasons or []}",
        f"[bold]Domains:[/bold] {bundle.link_domains or []}",
    ]

    # Add link mismatches if present (key phishing indicator)
    if bundle.link_mismatch_flags:
        lines.append(f"[bold red]Link Mismatches:[/bold red] {bundle.link_mismatch_flags}")

    # Add shortener flags if present
    if bundle.link_shortener_flags:
        lines.append(f"[bold yellow]Shorteners:[/bold yellow] {bundle.link_shortener_flags}")

    # Add first 200 chars
    if bundle.first_200_normalized:
        first_200 = _truncate(bundle.first_200_normalized, 200)
        lines.append(f"[bold]First 200:[/bold] \"{first_200}\"")

    # Add last 200 if different from first
    if bundle.last_200_normalized and bundle.last_200_normalized != bundle.first_200_normalized:
        last_200 = _truncate(bundle.last_200_normalized, 200)
        lines.append(f"[bold]Last 200:[/bold] \"{last_200}\"")

    content = "\n".join(lines)
    return Panel(
        content,
        title=f"[bold]{filename}[/bold]",
        border_style=tier_color,
    )


@app.command()
def pipeline(
    fixtures_dir: Path = typer.Option(
        ...,
        "--fixtures-dir",
        "-d",
        help="Directory containing JSON fixture files",
        exists=True,
        file_okay=False,
        dir_okay=True,
    ),
    verbose: bool = typer.Option(
        False,
        "--verbose",
        "-v",
        help="Show detailed output",
    ),
) -> None:
    """Run the inbox pipeline on JSON fixtures.

    Reads all *.json files from the fixtures directory, parses them as Gmail
    messages, runs the Pre-Gate analysis, and displays results.

    Example:
        arkai-inbox pipeline --fixtures-dir tests/fixtures/linkedin_spoof/
    """
    # Find all JSON files
    json_files = sorted(fixtures_dir.glob("*.json"))

    if not json_files:
        console.print(f"[yellow]No JSON files found in {fixtures_dir}[/yellow]")
        raise typer.Exit(1)

    console.print(f"[bold]Processing {len(json_files)} fixture files...[/bold]\n")

    # Create audit run
    audit = AuditLog.create_run()

    # Track results
    results: dict[str, list[str]] = {
        "PASS": [],
        "REVIEW": [],
        "QUARANTINE": [],
    }
    errors: list[tuple[str, str]] = []

    # Process each file
    for json_file in json_files:
        try:
            # Read and parse JSON
            with open(json_file, "r", encoding="utf-8") as f:
                raw_json = json.load(f)

            # Parse into EmailRecord
            email = parse_gmail_message(raw_json)

            # Create evidence bundle (runs Pre-Gate)
            bundle = create_evidence_bundle(email)

            # Log to audit
            log_pre_gate(
                audit,
                message_id=email.message_id,
                quarantine_tier=bundle.quarantine_tier,
                quarantine_reasons=bundle.quarantine_reasons,
                link_domains=bundle.link_domains,
                channel=email.channel,
            )

            # Track result
            results[bundle.quarantine_tier].append(json_file.name)

            # Display panel
            panel = _format_evidence_panel(json_file.name, bundle)
            console.print(panel)
            console.print()  # Spacing between panels

        except Exception as e:
            errors.append((json_file.name, str(e)))
            console.print(f"[red]Error processing {json_file.name}: {e}[/red]")
            if verbose:
                import traceback
                console.print(f"[dim]{traceback.format_exc()}[/dim]")

    # Print summary
    console.print()
    summary_content = [
        f"Processed: {len(json_files)} messages",
        f"[green]PASS: {len(results['PASS'])}[/green]  "
        f"[yellow]REVIEW: {len(results['REVIEW'])}[/yellow]  "
        f"[red]QUARANTINE: {len(results['QUARANTINE'])}[/red]",
    ]
    if errors:
        summary_content.append(f"[red]Errors: {len(errors)}[/red]")

    console.print(Panel(
        "\n".join(summary_content),
        title="[bold]Pipeline Summary[/bold]",
    ))

    # Print audit info
    summary = audit.get_run_summary()
    console.print(f"\n[dim]Audit log: {audit.run_dir}[/dim]")
    if verbose:
        console.print(f"[dim]Events logged: {summary['total_events']}[/dim]")


@app.command()
def gmail(
    query: str = typer.Option(
        "from:linkedin.com",
        "--query",
        "-q",
        help="Gmail search query",
    ),
    max_results: int = typer.Option(
        10,
        "--max",
        "-n",
        help="Maximum messages to fetch",
    ),
    verbose: bool = typer.Option(
        False,
        "--verbose",
        "-v",
        help="Show detailed output",
    ),
) -> None:
    """Run the inbox pipeline on live Gmail messages.

    Searches Gmail using the provided query and runs Pre-Gate analysis
    on each message. Requires prior authentication via 'arkai-gmail auth'.

    Examples:
        arkai-inbox gmail -q "from:linkedin.com" -n 5
        arkai-inbox gmail -q "from:notifications-noreply@linkedin.com"
        arkai-inbox gmail -q "from:linkedin.com newer_than:7d"
    """
    # Initialize Gmail client
    client = GmailClient()

    if not client.is_authenticated():
        console.print("[red]Not authenticated with Gmail.[/red]")
        console.print("Run 'arkai-gmail auth' first to authenticate.")
        raise typer.Exit(1)

    # Show who we're authenticated as
    try:
        email = client.get_user_email()
        console.print(f"[dim]Authenticated as: {email}[/dim]")
    except Exception as e:
        console.print(f"[red]Auth error: {e}[/red]")
        raise typer.Exit(1)

    # Search for messages
    console.print(f"[bold]Searching: {query}[/bold]")
    console.print(f"[dim]Max results: {max_results}[/dim]\n")

    try:
        message_refs = client.search_messages(query, max_results)
    except Exception as e:
        console.print(f"[red]Search failed: {e}[/red]")
        raise typer.Exit(1)

    if not message_refs:
        console.print("[yellow]No messages found matching query.[/yellow]")
        raise typer.Exit(0)

    console.print(f"[bold]Processing {len(message_refs)} messages...[/bold]\n")

    # Create audit run
    audit = AuditLog.create_run()

    # Track results
    results: dict[str, list[str]] = {
        "PASS": [],
        "REVIEW": [],
        "QUARANTINE": [],
    }
    errors: list[tuple[str, str]] = []

    # Process each message
    for i, ref in enumerate(message_refs, 1):
        message_id = ref["id"]

        try:
            # Fetch full message
            raw_message = client.get_message(message_id)

            # Parse into EmailRecord
            email_record: EmailRecord = parse_gmail_message(raw_message)

            # Create evidence bundle (runs Pre-Gate)
            bundle = create_evidence_bundle(email_record)

            # Log to audit
            log_pre_gate(
                audit,
                message_id=email_record.message_id,
                quarantine_tier=bundle.quarantine_tier,
                quarantine_reasons=bundle.quarantine_reasons,
                link_domains=bundle.link_domains,
                channel=email_record.channel,
            )

            # Track result
            results[bundle.quarantine_tier].append(message_id)

            # Display panel
            panel = _format_evidence_panel(f"[{i}/{len(message_refs)}] {message_id}", bundle)
            console.print(panel)
            console.print()

        except Exception as e:
            errors.append((message_id, str(e)))
            console.print(f"[red]Error processing {message_id}: {e}[/red]")
            if verbose:
                import traceback
                console.print(f"[dim]{traceback.format_exc()}[/dim]")

    # Print summary
    console.print()
    summary_content = [
        f"Processed: {len(message_refs)} messages",
        f"[green]PASS: {len(results['PASS'])}[/green]  "
        f"[yellow]REVIEW: {len(results['REVIEW'])}[/yellow]  "
        f"[red]QUARANTINE: {len(results['QUARANTINE'])}[/red]",
    ]
    if errors:
        summary_content.append(f"[red]Errors: {len(errors)}[/red]")

    console.print(Panel(
        "\n".join(summary_content),
        title="[bold]Gmail Pipeline Summary[/bold]",
    ))

    # Print audit info
    console.print(f"\n[dim]Audit log: {audit.run_dir}[/dim]")


@app.command()
def version() -> None:
    """Show version information."""
    from arkai_inbox import __version__
    console.print(f"arkai-inbox version {__version__}")


if __name__ == "__main__":
    app()
