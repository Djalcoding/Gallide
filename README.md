<div align="center">
# Gallide

![Release](https://img.shields.io/github/downloads/Djalcoding/Gallide/total?style=flat-square) 

Instead of repeatedly typing `cd`, `ls`, and `clear` to navigate your files, you can now write a single command: `g`

[Showcase](#showcase) · [Features](#features) · [Getting Started](#getting-started) · [Installation](#installation)
</div>

## Showcase
[Video](https://github.com/user-attachments/assets/438bba9a-3c0f-4b8f-ac58-af76d269a673)


## Features
   - **TUI for viewing files and directories**
   - **Vi controls support**
   - **fully customizable (with transparent background !)**
   - **cd-like usage**
   - **zoxide compatibility**

## Getting Started  
### **Usage:**
```
   g --help #displays help menu
   g #opens the GUI
   g <folder> #cd-like usage
   g -z <folder> #use zoxide
```

### **Configurating**
1. **Open ~/.config/gallide/gallide.conf**
   * *~/.config/gallide.conf is also a valid filepath*
2. **Refer to the [Documentation](not_done) to know what field to edit**
   * *note : it uses the [dparser format](https://github.com/Djalcoding/dparser.rs)*
3. 

## Installation
1. **Run the installation script**
   ```
   curl -fsSL https://github.com/Djalcoding/Gallide/releases/latest/download/install_gallide.sh| bash
   ```
2. **Add this to your .bashrc**
   ```
   eval "$(gallide --init)" # This will enable the g alias
   ```
**Prerequesites**
   - cargo
   - curl
   - bash (I think)
   - zoxide (with -z paramter; optional)


*note : this also works as an update script*


## What's next ?
- [ ] Add preview 


<br><br><br>
Powered by Rust 🦀
