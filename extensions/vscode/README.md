# Beejs VS Code Extension

This extension provides comprehensive support for the Beejs runtime in Visual Studio Code, offering enhanced JavaScript/TypeScript development with Beejs-specific features.

## Features

### 🐝 Language Support
- **Intelligent Code Completion**: Full autocomplete for Beejs APIs and runtime features
- **Hover Documentation**: Detailed information for Beejs runtime methods and properties
- **Syntax Highlighting**: Enhanced syntax highlighting for Beejs-specific features
- **Type Checking**: Optional TypeScript type checking for Beejs scripts

### 🔧 Debugging
- **Native Debug Adapter**: Full debugging support for Beejs runtime
- **Breakpoints**: Line, conditional, and function breakpoints
- **Stepping**: Step over, step into, and step out controls
- **Variable Inspection**: View and inspect variables during debugging
- **Call Stack**: Navigate the call stack during debugging

### ⚡ Performance Tools
- **Performance Profiling**: Built-in profiling tools to analyze script performance
- **Benchmark Suite**: Automated benchmarking for function performance
- **Memory Analysis**: Monitor memory usage during execution

### 🎯 Integration
- **CLI Integration**: Run Beejs commands directly from VS Code
- **Workspace Support**: Multi-folder workspace support
- **Configuration**: Customizable settings for Beejs runtime

## Installation

### From VS Code
1. Open VS Code
2. Go to Extensions (`Ctrl+Shift+X`)
3. Search for "Beejs Runtime Support"
4. Click Install

### From Package
```bash
npx @vscode/vsce package
code --install-extension beejs-tools-1.9.1.vsix
```

### From Source
```bash
git clone https://github.com/zh30/beejs.git
cd beejs/tools/vscode-extension
npm install
npm run compile
npx @vscode/vsce package
```

The extension is unlisted; install the `.vsix` locally. Marketplace publishing is not part of the 1.9.1 runtime release.

## Setup

### 1. Install Beejs Runtime
Ensure the `bee` binary is on your PATH. GitHub Release asset names match `.github/workflows/release-assets.yml`:

```bash
# macOS Apple Silicon
curl -fsSL https://github.com/zh30/beejs/releases/download/v1.9.1/bee-v1.9.1-aarch64-apple-darwin.tar.gz | tar -xz
# macOS Intel
curl -fsSL https://github.com/zh30/beejs/releases/download/v1.9.1/bee-v1.9.1-x86_64-apple-darwin.tar.gz | tar -xz
# Linux x64
curl -fsSL https://github.com/zh30/beejs/releases/download/v1.9.1/bee-v1.9.1-x86_64-unknown-linux-gnu.tar.gz | tar -xz
# Linux arm64
curl -fsSL https://github.com/zh30/beejs/releases/download/v1.9.1/bee-v1.9.1-aarch64-unknown-linux-gnu.tar.gz | tar -xz
# Windows x64
# bee-v1.9.1-x86_64-pc-windows-msvc.zip  (see install.ps1)

curl -fsSL https://bee.zhanghe.dev/install.sh | sh
brew install zh30/tap/bee
```

### 2. Configure Extension
1. Open Settings (`Ctrl+,`)
2. Search for "Beejs Runtime"
3. Configure settings:
   - `beejs.runtimePath`: Path to Beejs executable (default: `bee`)
   - `beejs.debugPort`: Debug port (default: `9229`)
   - `beejs.enableTypeChecking`: Enable TypeScript type checking (default: `true`)
   - `beejs.maxMemory`: Maximum memory allocation (default: `512m`)

## Usage

### Running Scripts
1. Open a JavaScript/TypeScript file
2. Press `F5` or right-click and select "Run Beejs Script"
3. Output appears in the "Beejs" output channel

### Debugging
1. Set breakpoints by clicking in the gutter
2. Press `F6` or right-click and select "Debug Beejs Script"
3. Use debugging controls to step through code

### Commands
- `Beejs: Run Script` - Run the current script
- `Beejs: Debug Script` - Debug the current script
- `Beejs: Show Performance Report` - Generate performance report
- `Beejs: Install Runtime` - Install Beejs runtime

### Keyboard Shortcuts
- `F5` - Run script
- `F6` - Debug script
- `Ctrl+Shift+P` then type "Beejs" - Show Beejs commands

## Configuration

### Workspace Settings
Create `.vscode/settings.json`:

```json
{
  "beejs.runtimePath": "/usr/local/bin/bee",
  "beejs.debugPort": 9229,
  "beejs.enableTypeChecking": true,
  "beejs.maxMemory": "512m"
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
      "name": "Debug Current File with bee",
      "runtimeExecutable": "bee",
      "runtimeArgs": ["run", "--inspect-brk", "--inspect-port", "9229"],
      "args": ["${file}"],
      "port": 9229
    },
    {
      "type": "node",
      "request": "attach",
      "name": "Attach to bee --inspect",
      "port": 9229
    }
  ]
}
```

Launch is equivalent to `bee run --inspect-brk --inspect-port 9229 ${file}`.

## API Reference

### Beejs Global API
The extension provides completion for the following Beejs APIs:

#### Runtime Execution
- `beejs.run(script)` - Execute a script
- `beejs.bundle(entry, output)` - Bundle scripts
- `beejs.test(pattern)` - Run tests

#### Performance
- `beejs.profile(fn)` - Profile function execution
- `beejs.benchmark(fn, iterations)` - Benchmark performance

#### TypeScript
- `beejs.compile(source, options)` - Compile TypeScript

## Development

### Building from Source
```bash
git clone https://github.com/beejs-team/beejs-vscode.git
cd beejs-vscode
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
git clone https://github.com/beejs-team/beejs-vscode.git
cd beejs-vscode
npm install
npm run compile
```

### Code Style
- Use TypeScript
- Follow existing code style
- Add JSDoc comments
- Write tests for new features

## Troubleshooting

### Beejs Not Found
- Verify Beejs is installed: `bee --version`
- Check `beejs.runtimePath` setting
- Try reinstalling Beejs

### Debug Not Working
- Verify debug port is not in use: `netstat -an | grep 9229`
- Check firewall settings
- Try a different debug port

### Performance Issues
- Increase `beejs.maxMemory` setting
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

- GitHub Issues: [https://github.com/beejs-team/beejs-vscode/issues](https://github.com/beejs-team/beejs-vscode/issues)
- Documentation: [https://beejs.dev/docs](https://beejs.dev/docs)
- Discord: [https://discord.gg/beejs](https://discord.gg/beejs)
