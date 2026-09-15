#!/usr/bin/env python3
"""Check the README table against measured baseline fields; never create labels."""
import json, pathlib, sys
root=pathlib.Path(__file__).resolve().parents[1]
s=json.loads((root/'benchmarks/baseline-v1/result.json').read_text())['summary']
def rate(key):
    r=s['rates'][key]; return f"{r['numerator']} / {r['denominator']}"
text=f'''<!-- benchmark:start -->
### B2IGE Verify Bench v1 / {s['total_cases']} explicit cases

| Measurement (on the benchmark corpus) | Observed |
|---|---:|
| Known bugs detected | {rate('true_bug_detection')} |
| False PASS | {s['false_pass']} |
| False FAIL | {s['false_fail']} |
| Verified reproduction | {rate('verified_reproduction')} |
| Hidden leakage observed | {s['hidden_leakage']} |
| Agent private-value leakage | {s['agent_leakage']} |
| Mutation adequacy: known benchmark mutants | {rate('Mutation adequacy on benchmark corpus')} |

All numbers above are on the benchmark corpus only. Bounded testing cannot establish complete correctness.
<!-- benchmark:end -->'''
p=root/'README.md'
if '--write' in sys.argv:
    content=p.read_text(); a=content.index('<!-- benchmark:start -->'); b=content.index('<!-- benchmark:end -->')+len('<!-- benchmark:end -->');p.write_text(content[:a]+text+content[b:])
else:
    assert text in p.read_text(), 'README benchmark differs from baseline'
    print('README measured benchmark snippet: PASS')
