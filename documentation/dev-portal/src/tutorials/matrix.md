# Matrix NymConnect Integration


Chat applications became an essential part of human communication. Matrix chat has end to end encryption on protocol level and Element app users can sort their communication into spaces and rooms. Now the Matrix communities can rely on network privacy as NymConnect supports Matrix chat protocol.

Currently there is no option in Matrix's Element client to set a socks5 proxy. In order to use Element via NymConnect users have to start it from the command-line. The setup is simple, for convenience a a keyboard shortcut setting can be easily done.


## Setup & Run

Make sure you have installed and started **[NymConnect](https://nymtech.net/developers/quickstart/nymconnect-gui.html)** on your desktop.

To then start Matrix's Element client via a Socks5 proxy connected to NymConnect, open terminal and run:

```sh
element-desktop --proxy-server=socks5://127.0.0.1:1080
```

## Optimise setup with a keybinding / alias

### Keybinding
An eloquent solution to avoid entering a command every time is to setup your keybinding. Open your settings, navigate to `Keyboard Shortcuts` and choose to `Set Custom Shortcut`. `Name` and `Shortcut` fields are up to your preference, to the `Command` line add:

```sh
element-desktop --proxy-server=socks5://127.0.0.1:1080
```
Make sure your `Shortcut` isn't already taken by something else in the menu.

An example can look like this.

![](../images/element_nym_keybind.png)

Alternatively you can add a keybinding via the CLI, using whatever config files you edit for your given desktop environment / window manager.

### Create an alias
If you prefer to simply shorten the length of the command (or all your keybindings are already taken) then you can simply create an alias for this long-winded command (this example aliases that command to the single word `element`, but you can replace it with whatever you like):

```sh
alias element="element-desktop --proxy-server=socks5://127.0.0.1:1080"
```

To make this alias persist, then add this to your `.bashrc` or `.zshrc` file (usually located in your `$HOME` directory) and `source` that file.

Now you can run Element throught the mixnet with a single-word command.
