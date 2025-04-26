const ChildProcess = require('child_process');
const Path = require('path');
const vscode = require('vscode');

class DefmtDecoder {
  constructor() {
    this.config = {};
    this.displayOutput = () => {};
    this.graphData = () => {};
    this.process = null;
  }


  init(config, displayOutput, graphData) {
    this.config = config;
    this.displayOutput = displayOutput;
    this.graphData = graphData;

    this.cwd = config.config.cwd || process.cwd();
    if (!config.config.executable) {
      throw new Error(JSON.stringify(config));
    }

    this.executable = Path.resolve(this.cwd, config.config.executable);

    this.process = ChildProcess.spawn(
      'defmt-print',
      ['-e', this.executable],
      { cwd: this.cwd }
    );

    this.process.stdout.on('data', (data) => {
      this.displayOutput(data.toString());
    });

    this.process.stderr.on('data', (data) => {
        this.displayOutput(`stderr: ${data.toString()}`);
    });

    this.process.on('error', (error) => {
        this.displayOutput(`Error: ${error.message}\n`);
    });
  }


  softwareEvent(_port, data) {
    if (!this.process || !this.process.stdin) {
      if (this.outputChannel) {
        this.displayOutput('Error: defmt-print process is not running.\n');
      }
      return;
    }
    let bufferInput = Buffer.isBuffer(data) ? data : Buffer.from(data);
    this.process.stdin.write(bufferInput);
  }

  outputLabel() {
    return this.config.label;
  }

  typeName() {
    return 'defmt-print';
  }
}

module.exports = { default: DefmtDecoder };