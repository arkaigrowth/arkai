#!/usr/bin/env python3
"""
arkai-tts: ElevenLabs Text-to-Speech CLI

Usage:
    arkai-tts "Hello, this is Claudia" --output hello.mp3
    arkai-tts --list-voices
    arkai-tts "Good morning" --voice "Rachel" --output morning.mp3
    arkai-tts --dry-run "Long text here..."  # Show cost estimate without calling API
"""

import sys
from pathlib import Path
from typing import Optional

import typer
from rich.console import Console
from rich.table import Table

from .elevenlabs_client import ElevenLabsTTS, ElevenLabsError

app = typer.Typer(
    name="arkai-tts",
    help="ElevenLabs Text-to-Speech CLI for Claudia",
    add_completion=False,
)
console = Console()


@app.command()
def main(
    text: Optional[str] = typer.Argument(
        None,
        help="Text to convert to speech",
    ),
    output: Optional[Path] = typer.Option(
        None,
        "--output", "-o",
        help="Output file path (e.g., output.mp3)",
    ),
    voice: str = typer.Option(
        "Rachel",
        "--voice", "-v",
        help="Voice name or ID",
    ),
    list_voices: bool = typer.Option(
        False,
        "--list-voices", "-l",
        help="List available voices",
    ),
    dry_run: bool = typer.Option(
        False,
        "--dry-run", "-n",
        help="Show cost estimate without calling API",
    ),
    stdin: bool = typer.Option(
        False,
        "--stdin",
        help="Read text from stdin",
    ),
):
    """
    Convert text to speech using ElevenLabs API.
    
    Examples:
        arkai-tts "Hello world" -o hello.mp3
        arkai-tts --list-voices
        echo "Hello" | arkai-tts --stdin -o hello.mp3
        arkai-tts --dry-run "Long text to estimate cost"
    """
    try:
        # Handle --list-voices
        if list_voices:
            _list_voices()
            return
        
        # Get text from argument or stdin
        if stdin:
            text = sys.stdin.read().strip()
        
        if not text:
            console.print("[red]Error:[/red] No text provided. Use --help for usage.")
            raise typer.Exit(1)
        
        # Handle --dry-run
        if dry_run:
            _dry_run(text)
            return
        
        # Require output path for generation
        if not output:
            console.print("[red]Error:[/red] --output is required for speech generation")
            raise typer.Exit(1)
        
        # Generate speech
        _generate(text, output, voice)
        
    except ElevenLabsError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(1)


def _list_voices():
    """List available voices."""
    try:
        tts = ElevenLabsTTS.from_env()
        voice_list = tts.list_voices()
        
        table = Table(title="Available Voices")
        table.add_column("Name", style="cyan")
        table.add_column("Voice ID", style="dim")
        table.add_column("Category", style="green")
        
        for v in voice_list:
            table.add_row(v["name"], v["voice_id"], v["category"])
        
        console.print(table)
        console.print(f"\n[dim]Total: {len(voice_list)} voices[/dim]")
        
    except ElevenLabsError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(1)


def _dry_run(text: str):
    """Show cost estimate without calling API."""
    estimate = ElevenLabsTTS.estimate_cost(text)
    
    console.print("\n[bold]Cost Estimate[/bold]")
    console.print(f"  Characters: {estimate['char_count']}")
    console.print(f"  Est. cost:  ${estimate['estimated_cost_usd']:.4f}")
    
    if "warning" in estimate:
        console.print(f"  [yellow]⚠️  {estimate['warning']}[/yellow]")
    
    console.print("\n[dim]Run without --dry-run to generate audio[/dim]")


def _generate(text: str, output: Path, voice: str):
    """Generate speech from text."""
    tts = ElevenLabsTTS.from_env()

    # Resolve voice name to ID if needed
    try:
        voice_id = tts.get_voice_id(voice)
    except ElevenLabsError:
        # If lookup fails, use as-is (might be an ID)
        voice_id = voice

    # Show estimate first
    estimate = ElevenLabsTTS.estimate_cost(text)
    console.print(f"[dim]Generating speech ({estimate['char_count']} chars)...[/dim]")

    # Generate
    result_path = tts.speak(text, str(output), voice_id=voice_id)

    console.print(f"[green]✓[/green] Generated: {result_path}")
    console.print(f"[dim]  Voice: {voice}[/dim]")
    console.print(f"[dim]  Size:  {result_path.stat().st_size:,} bytes[/dim]")


def cli():
    """Entry point for the CLI."""
    app()


if __name__ == "__main__":
    cli()
