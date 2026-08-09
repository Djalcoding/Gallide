#!/bin/bash
SCRIPT_DIR="$(\cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$HOME/.cargo/bin/gallide-bin"
DIRECTORY_REP="D'"
FILE_REP="F'"
found_conf=false
directory_path=""
explorer=cd
config_path=""

if test -f "$HOME/.config/gallide.conf"; then
    config_path="$HOME/.config/gallide.conf";
elif test -f "$HOME/.config/gallide/gallide.conf"; then
    config_path="$HOME/.config/gallide/gallide.conf";
fi

quit() {
    local code="${1:-0}"
    if [[ "${BASH_SOURCE[0]}" != "$0" ]]; then
        return 1
    else
        exit "$code"
    fi
}

if test -x "$BIN"; then
    :
elif test -f "/lib/gallide/gallide-bin"; then
    BIN="/lib/gallide/gallide-bin"
else
    echo "could not use gallide binary, try reinstalling gallide"
    echo "gallide should be in $BIN or /lib/gallide/gallide-bin"
    quit || return
fi

for arg in "$@"
do 
    if [ $found_conf = "true" ]; then
        config_path=$arg
        found_conf=false
        continue
    fi
    if [ $arg = "--init" ]; then
        echo "alias g='source $SCRIPT_DIR/gallide';"
        quit || return
    elif [ $arg = "-z" ] || [ $arg = "--zoxide" ]; then
        explorer=z
    elif [ $arg = "-h" ] || [ $arg = "--help" ]; then
        echo "Usage: g [OPTION]..."
        echo "Usage: g [OPTION]... [DIRECTORY]"
        echo "  -h | --help                displays this menu" 
        echo "  --init                     aliases g to gallide, \"\$(. gallide init)\"' should be put in your .bashrc"
        echo "  -c | --config <filepath>   set the config path"
        echo "  -z                         use zoxide instead of cd"
        echo "TUI controls : "
        echo "  k             move up"
        echo "  j             move down"
        echo "  l             enter selected directory"
        echo "  h             go back one directory"
        echo "  ESC           close interface on current directory"
        echo "  i             enable insert mode"
        echo "  Enter         open selected object"
        echo ""
        echo "Using a configuration :"
        echo "  configs located in $HOME/.config/gallide.conf or $HOME/.config/gallide/gallide.conf"
        echo "  will be automaticly detected and used unless overriden with the --config option"
        echo ""
        echo "Report bugs to : dbdevbugs@gmail.com"
        quit || return
    elif [ $arg = "--config" ] || [ $arg = "-c" ]; then
        found_conf=true
    elif [ "$arg" ]; then
        if [[ $directory_path != "" ]]; then
            echo "gallide: unknown argument '$arg'"
            quit || return
        fi
        directory_path="$arg"
    fi
done
if [ $found_conf = "true" ]; then
    echo "gallide: a filepath is expected after --config"
    quit 1 || return 1
fi

if [[ $directory_path != "" ]]; then
    if [[ "$(alias "$explorer" 2>/dev/null)" =~ ^alias[[:space:]]cd.*g ]]; then
        command "$explorer" "$directory_path"
    else 
        "$explorer" "$directory_path"
    fi
        quit || return
fi

LOGFILE="$(\mktemp /tmp/gallide_out.XXXXXX)"
if [[ $config_path == "" ]]; then
    "$BIN" 2>$LOGFILE # This actually runs the program
else
    "$BIN" "$config_path" 2>$LOGFILE # This actually runs the program
fi
OUTPUT=$(\cat "$LOGFILE")
ITEM_TYPE=${OUTPUT:0:2}
ITEM_PATH=${OUTPUT:2}
if [[ $ITEM_TYPE == $DIRECTORY_REP ]]; then
    \cd $ITEM_PATH
elif [[ "$ITEM_TYPE" == $FILE_REP ]]; then
    if [ -n "$EDITOR" ]; then
        "$EDITOR" "$ITEM_PATH"
    elif [ -n "$VISUAL" ]; then
        "$VISUAL" "$ITEM_PATH"
    else 
        cat "$ITEM_PATH"
    fi
else
    echo "Bash error; $LOGFILE contents : $OUTPUT" >&2
fi
