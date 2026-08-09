#!/bin/bash
INSTALLATION_FILE="$HOME/.local/bin/gallide"
CONFIG_DIR="$HOME/.config/gallide"
CONFIG_FILE="$CONFIG_DIR/gallide.conf"

if [[ "$1" == "--uninstall" ]]; then
    cargo uninstall gallide-bin
    rm $INSTALLATION_FILE
    echo "Completed uninstallation"
    exit 0;
fi

cargo install gallide-bin || { echo "Failed to fetch gallide binary from crates.io, exiting..."; exit; }
if ! test -f "$INSTALLATION_FILE"; then
    touch "$INSTALLATION_FILE" || { echo "Failed to create gallide.sh in bin, exiting..."; exit; }
fi
curl -L "https://github.com/Djalcoding/Gallide/releases/latest/download/gallide.sh" --output "$INSTALLATION_FILE" || { echo "Failed to fetch bash file from github,exiting..."; exit; }
FILESIZE=$(stat --printf="%s" $INSTALLATION_FILE)
if [ $FILESIZE = 0 ]; then
    echo "could not write to $INSTALLATION_FILE, exiting..."
    exit
fi

chmod +x "$INSTALLATION_FILE"

if ! test -d "$CONFIG_DIR"; then
    mkdir "$CONFIG_DIR"
fi
if ! test -f "$CONFIG_FILE" || [[ "$1" == "--rebuild-config" ]]; then
    touch "$CONFIG_FILE"
    curl -L "https://github.com/Djalcoding/Gallide/releases/latest/download/gallide.conf" --output "$CONFIG_FILE" || echo "Failed to fetch config file from github" 
    FILESIZE=$(stat --printf="%s" $CONFIG_FILE)
    if [ $FILESIZE = 0 ]; then
        echo "could not write to $CONFIG_FILE"
    fi
else
    echo "Skipping config file as it already exists"
fi

echo "Installation complete !";
echo "now, you probably want to add 'eval \"\$(. gallide init)\"' to your .bashrc"
