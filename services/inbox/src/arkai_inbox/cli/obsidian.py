"""Obsidian digest generator - converts audit JSONL to markdown."""

from datetime import datetime
from pathlib import Path

import typer
from rich.console import Console

from arkai_inbox.audit import AuditLog

app = typer.Typer()
console = Console()


def generate_digest(audit: AuditLog, vault_path: Path, inbox_root: str = "00-Inbox/Digest") -> Path:
    """Generate markdown digest from audit log.

    Args:
        audit: AuditLog instance
        vault_path: Path to Obsidian vault
        inbox_root: Relative path within vault for digests

    Returns:
        Path to generated markdown file
    """
    events = audit.read_events()
    summary = audit.get_run_summary()

    # Group events by message_id
    messages: dict[str, list[dict]] = {}
    for event in events:
        msg_id = event.get("message_id", "unknown")
        if msg_id not in messages:
            messages[msg_id] = []
        messages[msg_id].append(event)

    # Generate markdown
    today = datetime.now().strftime("%Y-%m-%d")
    lines = [
        f"# Inbox Digest - {today}",
        "",
        f"> Processed {len(messages)} messages",
        f"> PASS: {summary['by_tier']['PASS']} | REVIEW: {summary['by_tier']['REVIEW']} | QUARANTINE: {summary['by_tier']['QUARANTINE']}",
        "",
    ]

    # Section for each tier
    for tier in ["QUARANTINE", "REVIEW", "PASS"]:
        tier_messages = [
            (msg_id, evts) for msg_id, evts in messages.items()
            if any(e.get("quarantine_tier") == tier for e in evts)
        ]

        if not tier_messages:
            continue

        emoji = {"PASS": "✅", "REVIEW": "⚠️", "QUARANTINE": "🚫"}.get(tier, "")
        lines.append(f"## {emoji} {tier}")
        lines.append("")

        for msg_id, evts in tier_messages:
            pre_gate = next((e for e in evts if e.get("stage") == "pre_gate"), {})
            reasons = pre_gate.get("quarantine_reasons", [])
            domains = pre_gate.get("link_domains", [])

            lines.append(f"### `{msg_id[:12]}...`")
            if reasons:
                lines.append(f"- **Reasons:** {', '.join(reasons)}")
            if domains:
                lines.append(f"- **Domains:** {', '.join(domains[:5])}")
            lines.append("")

    # Footer
    lines.extend([
        "---",
        f"*Source: `{audit.run_dir}`*",
    ])

    # Write to vault
    digest_dir = vault_path / inbox_root
    digest_dir.mkdir(parents=True, exist_ok=True)

    output_path = digest_dir / f"{today}.md"
    output_path.write_text("\n".join(lines), encoding="utf-8")

    return output_path


@app.command()
def digest(
    run_dir: Path = typer.Option(
        None,
        "--run", "-r",
        help="Specific run directory (default: latest)",
    ),
    vault: Path = typer.Option(
        ...,
        "--vault", "-v",
        help="Path to Obsidian vault",
    ),
    inbox_root: str = typer.Option(
        "00-Inbox/Digest",
        "--inbox-root", "-i",
        help="Relative path within vault for digests",
    ),
) -> None:
    """Generate Obsidian markdown digest from audit log.

    Example:
        arkai-inbox digest --vault ~/Obsidian/vault-sandbox
    """
    # Find run directory
    if run_dir:
        if not run_dir.exists():
            console.print(f"[red]Run directory not found: {run_dir}[/red]")
            raise typer.Exit(1)
        audit = AuditLog.from_existing(run_dir)
    else:
        # Find latest run
        runs_dir = Path.home() / ".arkai" / "runs"
        if not runs_dir.exists():
            console.print("[red]No runs found. Run 'arkai-inbox gmail' first.[/red]")
            raise typer.Exit(1)

        inbox_runs = sorted(runs_dir.glob("inbox-*"), reverse=True)
        if not inbox_runs:
            console.print("[red]No inbox runs found.[/red]")
            raise typer.Exit(1)

        audit = AuditLog.from_existing(inbox_runs[0])
        console.print(f"[dim]Using latest run: {inbox_runs[0].name}[/dim]")

    # Expand vault path
    vault_expanded = vault.expanduser().resolve()
    if not vault_expanded.exists():
        console.print(f"[red]Vault not found: {vault_expanded}[/red]")
        raise typer.Exit(1)

    # Generate digest
    output = generate_digest(audit, vault_expanded, inbox_root)
    console.print(f"[green]✓ Digest generated:[/green] {output}")


if __name__ == "__main__":
    app()
