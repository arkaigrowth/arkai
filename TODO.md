# Future Enhancements

## ✅ Config File Support (Implemented)
`.arkai/config.yaml` is now supported with:
- Path configuration (home, library, content_types)
- Safety limits (max_steps, timeout, max_input_size)
- Fabric integration settings
- Config file discovery (searches current dir and parents)
- Env var overrides still work (highest priority)

## Potential Future Work

### Title Extraction from YouTube
Currently uses video ID as title. Could extract actual title from:
- yt-dlp metadata
- YouTube API
- Transcript content analysis

### Web Article Support
- Improve web content extraction (readability)
- Better title extraction from HTML

### Batch Processing
- Process multiple URLs from a file
- Playlist URL support

### Export/Import
- Export library to markdown
- Import from other knowledge bases
