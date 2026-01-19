#!/usr/bin/env python3
"""
render_transcript.py - Combine raw transcript + diarization + speakers into rendered view

This is a STUB implementing Chad's architecture:
- transcript_raw.md = canonical grounding (evidence validates here)
- diarization.jsonl = derived overlay (who spoke when)
- speakers.json = human name mapping (optional)
- transcript.md = rendered view (rebuildable)

Usage:
    python render_transcript.py ./library/youtube/Video\ Title/

    # Or with explicit paths:
    python render_transcript.py \
        --raw transcript_raw.md \
        --diarization diarization.jsonl \
        --speakers speakers.json \
        --output transcript.md
"""

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Optional


def parse_timestamp(timestamp_str: str) -> float:
    """Convert [HH:MM:SS] or [MM:SS] to seconds."""
    match = re.match(r'\[(\d+):(\d+):(\d+)\]', timestamp_str)
    if match:
        h, m, s = map(int, match.groups())
        return h * 3600 + m * 60 + s

    match = re.match(r'\[(\d+):(\d+)\]', timestamp_str)
    if match:
        m, s = map(int, match.groups())
        return m * 60 + s

    return 0.0


def load_diarization(path: Path) -> list[dict]:
    """Load diarization.jsonl file."""
    segments = []
    if path.exists():
        with open(path) as f:
            for line in f:
                line = line.strip()
                if line:
                    segments.append(json.loads(line))
    return segments


def load_speakers(path: Path) -> dict[str, str]:
    """Load speakers.json file."""
    if path.exists():
        with open(path) as f:
            data = json.load(f)
            # Filter out non-mapping keys like "notes"
            return {k: v for k, v in data.items() if k.startswith("SPEAKER_")}
    return {}


def find_speaker_at_timestamp(diarization: list[dict], timestamp: float) -> Optional[str]:
    """Find which speaker is talking at a given timestamp."""
    for segment in diarization:
        if segment["start"] <= timestamp <= segment["end"]:
            return segment["speaker"]
    return None


def render_transcript(
    raw_content: str,
    diarization: list[dict],
    speakers: dict[str, str]
) -> str:
    """
    Render transcript with speaker labels.

    Input (transcript_raw.md):
        [00:00:02] Hey, what's up?
        [00:00:05] Yeah, that's interesting.

    Output (transcript.md):
        [00:00:02] [Daniel] Hey, what's up?
        [00:00:05] [Guest] Yeah, that's interesting.
    """
    lines = raw_content.split('\n')
    rendered_lines = []

    # Track speakers for header
    found_speakers = set()

    for line in lines:
        # Check if line starts with timestamp
        match = re.match(r'^(\[\d+:\d+:\d+\])\s*(.*)$', line)
        if match:
            timestamp_str, text = match.groups()
            timestamp = parse_timestamp(timestamp_str)

            # Find speaker at this timestamp
            speaker_label = find_speaker_at_timestamp(diarization, timestamp)

            if speaker_label:
                found_speakers.add(speaker_label)
                # Resolve to human name if available
                speaker_name = speakers.get(speaker_label, speaker_label)
                rendered_lines.append(f"{timestamp_str} [{speaker_name}] {text}")
            else:
                # No diarization data for this timestamp
                rendered_lines.append(line)
        else:
            # Non-timestamp line (headers, blank lines, etc.)
            rendered_lines.append(line)

    # Add speakers to header if we have any
    if found_speakers and "**Speakers**:" not in raw_content:
        # Find the --- separator and insert speakers after it
        for i, line in enumerate(rendered_lines):
            if line.strip() == "---" and i > 0:
                speaker_names = [speakers.get(s, s) for s in sorted(found_speakers)]
                rendered_lines.insert(i, f"**Speakers**: {', '.join(speaker_names)}")
                break

    return '\n'.join(rendered_lines)


def main():
    parser = argparse.ArgumentParser(
        description="Render transcript with speaker labels from diarization overlay"
    )
    parser.add_argument(
        "library_dir",
        nargs="?",
        help="Library content directory (contains transcript_raw.md, etc.)"
    )
    parser.add_argument("--raw", help="Path to transcript_raw.md")
    parser.add_argument("--diarization", help="Path to diarization.jsonl")
    parser.add_argument("--speakers", help="Path to speakers.json")
    parser.add_argument("--output", "-o", help="Output path for transcript.md")

    args = parser.parse_args()

    # Determine paths
    if args.library_dir:
        base = Path(args.library_dir)
        raw_path = base / "transcript_raw.md"
        # Fallback to fetch.md for backward compatibility
        if not raw_path.exists():
            raw_path = base / "fetch.md"
        diarization_path = base / "diarization.jsonl"
        speakers_path = base / "speakers.json"
        output_path = base / "transcript.md"
    else:
        raw_path = Path(args.raw) if args.raw else Path("transcript_raw.md")
        diarization_path = Path(args.diarization) if args.diarization else Path("diarization.jsonl")
        speakers_path = Path(args.speakers) if args.speakers else Path("speakers.json")
        output_path = Path(args.output) if args.output else Path("transcript.md")

    # Validate raw transcript exists
    if not raw_path.exists():
        print(f"Error: Raw transcript not found: {raw_path}", file=sys.stderr)
        print("The canonical transcript (transcript_raw.md or fetch.md) must exist.", file=sys.stderr)
        sys.exit(1)

    # Load files
    raw_content = raw_path.read_text()
    diarization = load_diarization(diarization_path)
    speakers = load_speakers(speakers_path)

    # Status output
    print(f"Raw transcript: {raw_path}")
    print(f"Diarization segments: {len(diarization)}")
    print(f"Speaker mappings: {len(speakers)}")

    if not diarization:
        print("\nNote: No diarization data found. Output will match raw transcript.")
        print("To add diarization, create diarization.jsonl with speaker segments.")

    # Render
    rendered = render_transcript(raw_content, diarization, speakers)

    # Write output
    output_path.write_text(rendered)
    print(f"\nRendered transcript written to: {output_path}")

    # Show sample
    print("\n--- Sample output (first 10 lines) ---")
    for line in rendered.split('\n')[:10]:
        print(line)


if __name__ == "__main__":
    main()
