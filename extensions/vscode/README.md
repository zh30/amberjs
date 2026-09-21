# Amber VS Code Extension

This extension provides comprehensive support for the Amber runtime in Visual Studio Code, offering enhanced JavaScript/TypeScript development with Amber-specific features.

## Features

### 🐝 Language Support
- **Intelligent Code Completion**: Full autocomplete for Amber APIs and runtime features
- **Hover Documentation**: Detailed information for Amber runtime methods and properties
- **Syntax Highlighting**: Enhanced syntax highlighting for Amber-specific features
- **Type Checking**: Optional TypeScript type checking for Amber scripts

### 🔧 Debugging
- **Native Debug Adapter**: Full debugging support for Amber runtime
- **Breakpoints**: Line, conditional, and function breakpoints
- **Stepping**: Step over, step into, and step out controls
- **Variable Inspection**: View and inspect variables during debugging
- **Call Stack**: Navigate the call stack during debugging

### ⚡ Performance Tools
- **Performance Profiling**: Built-in profiling tools to analyze script performance
- **Benchmark Suite**: Automated benchmarking for function performance
- **Memory Analysis**: Monitor memory usage during execution

### 🎯 Integration
- **CLI Integration**: Run Amber commands directly from VS Code
- **Workspace Support**: Multi-folder workspace support
- **Configuration**: Customizable settings for Amber runtime

## Installation

### From VS Code
1. Open VS Code
2. Go to Extensions (`Ctrl+Shift+X`)
3. Search for "Amber Runtime Support"
4. Click Install

### From Package
```bash
npx @vscode/vsce package
code --install-extension amberjs-tools-1.9.1.vsix
```

### From Source
```bash
git clone https://github.com/zh30/amberjs.git
cd amberjs/tools/vscode-extension
npm install
npm run compile
npx @vscode/vsce package
```

The extension is unlisted; install the `.vsix` locally. Marketplace publishing is not part of the 1.9.1 runtime release.

## Setup

### 1. Install Amber Runtime
Ensure the `amber` binary is on your PATH. GitHub Release asset names match `.github/workflows/release-assets.yml`:

```bash
# macOS Apple Silicon
curl -fsSL https://github.com/zh30/amberjs/releases/download/v1.9.1/amber-v1.9.1-aarch64-apple-darwin.tar.gz | tar -xz
# macOS Intel
curl -fsSL https://github.com/zh30/amberjs/releases/download/v1.9.1/amber-v1.9.1-x86_64-apple-darwin.tar.gz | tar -xz
# Linux x64
curl -fsSL https://github.com/zh30/amberjs/releases/download/v1.9.1/amber-v1.9.1-x86_64-unknown-linux-gnu.tar.gz | tar -xz
# Linux arm64
curl -fsSL https://github.com/zh30/amberjs/releases/download/v1.9.1/amber-v1.9.1-aarch64-unknown-linux-gnu.tar.gz | tar -xz
# Windows x64
# amber-v1.9.1-x86_64-pc-windows-msvc.zip  (see install.ps1)

curl -fsSL https://amber.zhanghe.dev/install.sh | sh
brew install zh30/tap/amber
```

### 2. Configure Extension
1. Open Settings (`Ctrl+,`)
2. Search for "Amber Runtime"
3. Configure settings:
   - `amberjs.runtimePath`: Path to Amber executable (default: `amber`)
   - `amberjs.debugPort`: Debug port (default: `9229`)
   - `amberjs.enableTypeChecking`: Enable TypeScript type checking (default: `true`)
   - `amberjs.maxMemory`: Maximum memory allocation (default: `512m`)

## Usage

### Running Scripts
1. Open a JavaScript/TypeScript file
2. Press `F5` or right-click and select "Run Amber Script"
3. Output appears in the "Amber" output channel

### Debugging
1. Set breakpoints by clicking in the gutter
2. Press `F6` or right-click and select "Debug Amber Script"
3. Use debugging controls to step through code

### Commands
- `Amber: Run Script` - Run the current script
- `Amber: Debug Script` - Debug the current script
- `Amber: Show Performance Report` - Generate performance report
- `Amber: Install Runtime` - Install Amber runtime

### Keyboard Shortcuts
- `F5` - Run script
- `F6` - Debug script
- `Ctrl+Shift+P` then type "Amber" - Show Amber commands

## Configuration

### Workspace Settings
Create `.vscode/settings.json`:

```json
{
  "amberjs.runtimePath": "/usr/local/bin/amber",
  "amberjs.debugPort": 9229,
  "amberjs.enableTypeChecking": true,
  "amberjs.maxMemory": "512m"
}
```

### Launch Configuration
Create `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "node",
      "request": "launch",
      "name": "Debug Current File with amber",
      "runtimeExecutable": "amber",
      "runtimeArgs": ["run", "--inspect-brk", "--inspect-port", "9229"],
      "args": ["${file}"],
      "port": 9229
    },
    {
      "type": "node",
      "request": "attach",
      "name": "Attach to amber --inspect",
      "port": 9229
    }
  ]
}
```

Launch is equivalent to `amber run --inspect-brk --inspect-port 9229 ${file}`.

## API Reference

### Amber Global API
The extension provides completion for the following Amber APIs:

#### Runtime Execution
- `amberjs.run(script)` - Execute a script
- `amberjs.bundle(entry, output)` - Bundle scripts
- `amberjs.test(pattern)` - Run tests

#### Performance
- `amberjs.profile(fn)` - Profile function execution
- `amberjs.benchmark(fn, iterations)` - Benchmark performance

#### TypeScript
- `amberjs.compile(source, options)` - Compile TypeScript

## Development

### Building from Source
```bash
git clone https://github.com/amberjs-team/amberjs-vscode.git
cd amberjs-vscode
npm install
npm run compile
```

### Running Tests
```bash
npm test
```

### Packaging Extension
```bash
npm run vscode:prepublish
vsce package
```

### Debugging Extension
1. Press `F5` in VS Code to launch extension development host
2. The extension will be loaded in a new window
3. Use the debugger to debug the extension itself

## Architecture

### Language Service
The extension uses the Language Server Protocol (LSP) to provide:
- Code completion
- Hover information
- Diagnostics
- Code actions

### Debug Adapter
Implements the Debug Adapter Protocol (DAP) to provide:
- Launch and attach debugging
- Breakpoint management
- Stepping controls
- Variable inspection

### Commands
VS Code commands for:
- Script execution
- Debugging
- Performance analysis
- Runtime installation

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

### Development Setup
```bash
git clone https://github.com/amberjs-team/amberjs-vscode.git
cd amberjs-vscode
npm install
npm run compile
```

### Code Style
- Use TypeScript
- Follow existing code style
- Add JSDoc comments
- Write tests for new features

## Troubleshooting

### Amber Not Found
- Verify Amber is installed: `amber --version`
- Check `amberjs.runtimePath` setting
- Try reinstalling Amber

### Debug Not Working
- Verify debug port is not in use: `netstat -an | grep 9229`
- Check firewall settings
- Try a different debug port

### Performance Issues
- Increase `amberjs.maxMemory` setting
- Check available system memory
- Disable type checking if not needed

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Changelog

### 0.1.0
- Initial release
- Language support (completion, hover, diagnostics)
- Debug adapter with full debugging capabilities
- Performance profiling tools
- CI/CD integration templates

## Support

- GitHub Issues: [https://github.com/amberjs-team/amberjs-vscode/issues](https://github.com/amberjs-team/amberjs-vscode/issues)
- Documentation: [https://amberjs.dev/docs](https://amberjs.dev/docs)
- Discord: [https://discord.gg/amberjs](https://discord.gg/amberjs)
