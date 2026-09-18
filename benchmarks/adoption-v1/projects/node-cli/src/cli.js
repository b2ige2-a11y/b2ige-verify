const {status} = require('./status');
const command = process.argv[2] || '--help';
if (command === '--help') process.stdout.write('status-cli base');
else if (command === 'base') process.stdout.write(status());
else { process.stderr.write('unknown command\n'); process.exitCode = 2; }
