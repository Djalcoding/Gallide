#!/bin/bash
INSTALLATION_FILE="$HOME/.local/bin/gallide"

cargo install gallide-bin || { echo "Failed to fetch gallide binary from crates.io, exiting..."; exit; }
if ! test -f "$INSTALLATION_FILE"; then
    touch "$INSTALLATION_FILE" || { echo "Failed to create gallide.sh in bin, exiting..."; exit; }
fi
curl https://github.com/Djalcoding/Gallide/releases/latest/download/gallide.sh --output "$INSTALLATION_FILE" || { echo "Failed to fetch bash from github,exiting..."; exit; }
FILESIZE=$(stat --printf="%s" $INSTALLATION_FILE)
echo $FILESIZE
if [ $FILESIZE = 0 ]; then
    echo "could not write to $INSTALLATION_FILE, exiting..."
    exit
fi

echo "Installation complete !";
echo "now, you probably want to add 'eval \"\$(. gallide init)\"' to your .bashrc"
