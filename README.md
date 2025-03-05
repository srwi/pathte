# Pathte <img src="resources/icon.svg" align="right" width="20%"/>
<a href="#"><img src="https://img.shields.io/badge/platform-windows-blue"/></a> <a href="#"><img src="https://img.shields.io/badge/license-MIT-green"/></a>

Have you ever pasted a file path into a terminal, only to find it's in the wrong format? Worry no more!

Pathte is a handy Windows utility that effortlessly converts file paths between Windows, Unix, and WSL formats as you paste. Enhance your workflow with Pathte by detecting file paths in your clipboard and instantly converting them to the desired format, making your pasting experience smoother and more efficient.

<p align="center">
    <img src="resources/demo.gif" alt="Pathte Demo" width="60%"/>
</p>

## How It Works

When you press <kbd>Ctrl</kbd>+<kbd>V</kbd> to paste, Pathte checks if your clipboard contains a valid file path. If so, a popup menu appears with the available format options. You can then press <kbd>V</kbd> to cycle through the formats, and release <kbd>Ctrl</kbd> to paste the path in your selected format:

| Format        | Example                   |
|---------------|---------------------------|
| Windows       | `C:\folder\file.txt`      |
| Unix-like     | `C:/folder/file.txt`      |
| WSL           | `/mnt/c/folder/file.txt`  |

Whenever there is _no_ file path in your clipboard, Pathte will act like it's not even there.

## License

This project is licensed under the [MIT](https://github.com/srwi/pathte/blob/master/LICENSE) License.