#!/bin/bash
# Olek's Obsidian Vault Quick Setup
# Run this AFTER installing Obsidian

echo "🚀 Olek's Obsidian Vault Setup"
echo "=============================="
echo ""

# Check if Obsidian is installed
if [ -d "/Applications/Obsidian.app" ]; then
    echo "✅ Obsidian detected"
else
    echo "❌ Obsidian not found. Install from: https://obsidian.md/download"
    exit 1
fi

# Default vault location
VAULT_DIR="$HOME/Documents/obsidian-olek-vault"

echo ""
echo "This will set up your vault at: $VAULT_DIR"
echo ""
read -p "Press Enter to continue (or Ctrl+C to cancel)..."

# Create vault directory if needed
if [ ! -d "$VAULT_DIR" ]; then
    mkdir -p "$VAULT_DIR"
    echo "✅ Created vault directory"
fi

# Copy contents (assuming script is run from unzipped folder)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cp -R "$SCRIPT_DIR"/* "$VAULT_DIR/" 2>/dev/null
cp -R "$SCRIPT_DIR"/.obsidian "$VAULT_DIR/" 2>/dev/null

echo "✅ Copied vault files"

echo ""
echo "=============================="
echo "📋 NEXT STEPS (do these manually):"
echo "=============================="
echo ""
echo "1. Open Obsidian"
echo "2. Click 'Open folder as vault'"
echo "3. Select: $VAULT_DIR"
echo "4. Go to Settings → Community Plugins"
echo "5. Turn OFF 'Restricted Mode'"
echo "6. Click 'Browse' and install these plugins:"
echo ""
echo "   REQUIRED (install in order):"
echo "   • Homepage"
echo "   • Calendar"  
echo "   • Periodic Notes"
echo "   • Templater"
echo "   • Dataview"
echo "   • Recent Files"
echo "   • Omnisearch"
echo "   • Hover Editor"
echo "   • Quick Explorer"
echo "   • Auto Note Mover"
echo ""
echo "   OPTIONAL (nice to have):"
echo "   • Smart Connections"
echo "   • Tag Wrangler"
echo "   • Colorful Folders"
echo "   • Copilot"
echo ""
echo "7. After installing, RESTART Obsidian"
echo "8. Daily note should auto-open! 🎉"
echo ""
echo "=============================="
echo "⌨️  KEY HOTKEYS:"
echo "   Cmd+D  → Open today's daily note"
echo "   Cmd+O  → Quick switcher"
echo "   Cmd+T  → Insert template"
echo "=============================="
