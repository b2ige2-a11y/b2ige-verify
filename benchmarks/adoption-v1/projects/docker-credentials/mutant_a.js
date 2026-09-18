// Same deliberate mutant as the existing P8 target constructor.
(()=>{const expiry=Number(process.argv[2]);const now=Number(process.env.LOGICAL_NOW);
const rejected = false;
process.stdout.write(rejected?'rejected\n':'session created\n');process.exit(rejected?1:0);})();
