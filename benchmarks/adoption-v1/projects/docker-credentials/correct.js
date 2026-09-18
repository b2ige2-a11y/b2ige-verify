// Same credential predicate and byte output as the existing P8 target constructor.
(()=>{const expiry=Number(process.argv[2]);const now=Number(process.env.LOGICAL_NOW);
const rejected = expiry <= now;
process.stdout.write(rejected?'rejected\n':'session created\n');process.exit(rejected?1:0);})();
