//! Loopback-only, read-only HTML; every navigation revalidates the root and source.
use crate::{badge, load, pretty, VerifiedReport};
use std::{
    collections::{BTreeSet, VecDeque},
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};
use verify_core::behavior::BehaviorAuthorization;
use verify_evidence::store::EvidenceStore;

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn pre(v: &impl serde::Serialize) -> String {
    format!("<pre tabindex=\"0\">{}</pre>", escape(&pretty(v)))
}
fn detail(label: &str, content: &str) -> String {
    format!(
        "<details><summary>{}</summary>{content}</details>",
        escape(label)
    )
}
fn page(content: &str) -> String {
    format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>B2IGE · Verification report</title><style>{}</style></head><body><main><header>B2IGE <span>VERIFY / LOCAL REPORT</span></header>{content}<footer>Read-only projection · Revalidated on navigation · No exhaustive proof</footer></main></body></html>",include_str!("viewer.css"))
}
pub fn render(report: &VerifiedReport, prefix: &str, root_id: &str) -> String {
    let d = report.document();
    let mut body=format!("<nav aria-label=\"Report navigation\"><a href=\"{}/{}\">Source report</a></nav><section class=\"verdict v{}\"><p class=\"badge\">{}</p><h1>{}</h1></section>",escape(prefix),escape(root_id),d.verdict.exit_code(),if d.blindtest.is_some() && d.verdict == verify_core::Verdict::Fail {"✕ NOT READY"} else {badge(d.verdict)},escape(&d.headline).replace('\n',"<br>"));
    if d.verdict != verify_core::Verdict::Pass {
        body.push_str(&format!("<h2>Reason</h2><p>{}</p>", escape(&d.reason)));
    }
    if let Some(f) = &d.primary_failure {
        body.push_str(&format!("<section aria-label=\"Primary failure\"><h2>#1 · {}</h2><div class=\"comparison\"><section><h3>Expected</h3>{}</section><section><h3>Observed</h3>{}</section></div></section>",escape(&pretty(&f.divergence.observable)),pre(&f.divergence.before),pre(&f.divergence.after)));
    }
    if let Some(s) = &d.sideeffect {
        body.push_str(&format!("<section aria-label=\"Committed effect summary\"><p>Attempts <strong>{}</strong> · Committed <strong>{}</strong></p>",s.attempts,s.committed.map(|n|n.to_string()).unwrap_or_else(||"unknown".into())));
        for e in &s.effects {
            body.push_str(&format!("<h2>{}</h2><div class=\"comparison\"><section><h3>Expected</h3><p>{}</p></section><section><h3>Observed</h3><p>{} × {}</p></section></div>",escape(&e.effect_id),escape(crate::sideeffect::expectation(e.expectation)),escape(&e.effect_id),e.committed.map(|n|n.to_string()).unwrap_or_else(||"unknown".into())));
        }
        if let Some(x) = &s.counterexample {
            body.push_str(&format!(
                "<h2>{}</h2><p>{}</p><ol>",
                if x.status == verify_core::sideeffect::ReductionStatus::LocallyMinimized {
                    "Locally minimized reproduction"
                } else {
                    "Recorded reproduction (no minimality claim)"
                },
                match x.status {
                    verify_core::sideeffect::ReductionStatus::LocallyMinimized =>
                        "Verified by rerunning the target",
                    verify_core::sideeffect::ReductionStatus::BudgetExhausted =>
                        "Reduction budget exhausted",
                    verify_core::sideeffect::ReductionStatus::Unreproduced =>
                        "Original failure did not reproduce",
                }
            ));
            for step in &x.steps {
                body.push_str(&format!("<li>{}</li>", escape(step)));
            }
            body.push_str("</ol>");
        }
        body.push_str("</section>");
    }
    if let Some(b) = &d.blindtest {
        if d.verdict == verify_core::Verdict::Pass {
            body.push_str("<p>No failures detected<br>within the executed hidden suite</p>");
        }
        body.push_str(&format!(
            "<p>Hidden checks · {}<br>Isolation · {}</p>",
            b.hidden_checks,
            escape(&b.isolation)
        ));
        for f in &b.failures {
            body.push_str(&format!("<section aria-label=\"Hidden invariant failure\"><h2>{}</h2><div class=\"comparison\"><section><h3>Expected</h3><p>{}</p></section><section><h3>Observed</h3><p>{}</p></section></div><h3>Reproduction</h3><ol>",escape(&f.invariant),escape(&f.expected),escape(&f.observed)));
            for step in &f.reproduction {
                body.push_str(&format!("<li>{}</li>", escape(step)));
            }
            body.push_str("</ol></section>");
        }
    }
    let mut supporting = String::new();
    if let Some(r) = &d.reproduction {
        let mut repro = format!("<section><h2>{}</h2>", escape(&r.label));
        if let (Some(a), Some(b)) = (r.original_assignments, r.retained_assignments) {
            repro.push_str(&format!("<p>Original reproduction · {a} assignments</p><p>↓</p><p>Retained reproduction · {b} assignments</p>"));
        }
        if let Some(status) = r.reduction_status {
            repro.push_str(&format!(
                "<p class=\"status\">{}</p>",
                escape(&pretty(&status))
            ));
        }
        repro.push_str(&format!(
            "<h3>Arguments</h3>{}<h3>Environment assignments</h3>{}",
            pre(&r.experiment.case.args),
            pre(&r.experiment.case.environment)
        ));
        repro.push_str(&detail("Recorded experiment inputs", &pre(r)));
        repro.push_str("</section>");
        if d.verdict == verify_core::Verdict::Pass {
            supporting.push_str(&repro);
        } else {
            body.push_str(&repro);
        }
    }
    if !d.other_failures.is_empty() {
        let mut others = String::new();
        for (i, f) in d.other_failures.iter().enumerate() {
            others.push_str(&format!(
                "<h3>#{} · {}</h3>{}",
                i + 2,
                escape(&f.comparison_id),
                pre(f)
            ));
        }
        supporting.push_str(&detail(
            &format!("Other failures · {}", d.other_failures.len()),
            &others,
        ));
    }
    supporting.push_str(&detail(
        &format!("Evidence · {}", d.evidence_summaries.len()),
        &pre(&d.evidence_summaries),
    ));
    if let Some(s) = &d.sideeffect {
        supporting.push_str(&detail(
            &format!("Timeline · {} events", s.timeline.len()),
            &pre(&s.timeline),
        ));
        supporting.push_str(&detail(
            &format!(
                "Runs · {}",
                s.schedules.iter().map(|s| s.attempts).sum::<usize>()
            ),
            &pre(&s.schedules),
        ));
    }
    if let Some(b) = &d.blindtest {
        supporting.push_str(&detail("Isolation", &pre(&b.isolation_attestations)));
        supporting.push_str(&detail(
            &format!("Runs · {}", b.runs.as_array().map_or(0, Vec::len)),
            &pre(&b.runs),
        ));
        supporting.push_str(&detail(
            "Hidden details · trusted human view",
            &pre(&b.hidden_details),
        ));
        supporting.push_str(&detail("Suite quality", &pre(&b.quality)));
    }
    if d.sideeffect.is_none() && d.blindtest.is_none() {
        supporting.push_str(&detail(
            &format!("Runs · {}", d.run_references.len()),
            &pre(&d.run_references),
        ));
    }
    supporting.push_str(&detail("Coverage", &pre(&d.coverage)));
    supporting.push_str(&detail(
        "Limitations",
        &format!(
            "{}<h3>Replayability</h3>{}",
            pre(&d.limitations),
            pre(&d.replayability)
        ),
    ));
    if !d.related_artifacts.is_empty() {
        let links = d
            .related_artifacts
            .iter()
            .map(|a| {
                format!(
                    "<li><a href=\"{}/{}\">{}</a></li>",
                    escape(prefix),
                    escape(&a.artifact_id),
                    escape(&a.artifact_id)
                )
            })
            .collect::<String>();
        supporting.push_str(&detail(
            &format!("Related verified artifacts · {}", d.related_artifacts.len()),
            &format!("<ul>{links}</ul>"),
        ));
    }
    supporting.push_str(&detail(
        "Raw artifact",
        &format!(
            "<h3>Source identity</h3>{}{}",
            pre(&d.source),
            pre(report.raw())
        ),
    ));
    if d.verdict == verify_core::Verdict::Pass {
        body.push_str(&detail("Details", &supporting));
    } else {
        body.push_str(&supporting);
    }
    page(&body)
}
pub fn error_page() -> String {
    page("<section class=\"verdict v3\"><p class=\"badge\">! ERROR</p><h1>Verification could not complete</h1><p>Source or linked evidence could not be verified. No verified report is available.</p></section>")
}

pub struct Viewer {
    listener: TcpListener,
    store: EvidenceStore,
    auth: BehaviorAuthorization,
    root_id: String,
    token: String,
}
impl Viewer {
    pub fn bind(store: EvidenceStore, auth: BehaviorAuthorization, id: &str) -> io::Result<Self> {
        load(&store, id, &auth)?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let mut bytes = [0u8; 24];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Self {
            listener,
            store,
            auth,
            root_id: id.into(),
            token,
        })
    }
    pub fn url(&self) -> io::Result<String> {
        Ok(format!(
            "http://{}/{}/{}",
            self.listener.local_addr()?,
            self.token,
            self.root_id
        ))
    }
    pub fn serve(self) -> io::Result<()> {
        for stream in self.listener.incoming() {
            let mut stream = stream?;
            let _ = self.handle(&mut stream);
        }
        Ok(())
    }
    fn handle(&self, stream: &mut TcpStream) -> io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let mut request = Vec::new();
        let mut byte = [0];
        while request.len() < 8192 {
            if stream.read(&mut byte)? == 0 {
                break;
            }
            request.push(byte[0]);
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8_lossy(&request);
        let (status, body) = self.response(&request);
        write!(stream,"HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",body.len())
    }
    fn response(&self, request: &str) -> (&'static str, String) {
        let host = self
            .listener
            .local_addr()
            .expect("bound socket")
            .to_string();
        let lines: Vec<_> = request.split("\r\n").collect();
        let hosts: Vec<_> = lines
            .iter()
            .filter_map(|l| l.split_once(':'))
            .filter(|(n, _)| n.eq_ignore_ascii_case("host"))
            .collect();
        if hosts.len() != 1 || hosts[0].1.trim() != host {
            return ("403 Forbidden", error_page());
        }
        let parts: Vec<_> = lines[0].split_whitespace().collect();
        if parts.len() != 3 || parts[0] != "GET" {
            return ("405 Method Not Allowed", error_page());
        }
        let prefix = format!("/{}/", self.token);
        let Some(id) = parts[1].strip_prefix(&prefix) else {
            return ("404 Not Found", error_page());
        };
        let result = (|| {
            let root = load(&self.store, &self.root_id, &self.auth)?;
            // Only the root's verified product graph is navigable. Never arbitrary files.
            let mut visited = BTreeSet::new();
            let mut queue = VecDeque::from([root]);
            let selected = loop {
                let current = queue
                    .pop_front()
                    .ok_or_else(|| io::Error::other("not linked"))?;
                if current.document().source.artifact_id == id {
                    break current;
                }
                for link in &current.document().related_artifacts {
                    if visited.insert(link.artifact_id.clone()) {
                        let child = load(&self.store, &link.artifact_id, &self.auth)?;
                        if child.document().source.integrity_hash != link.integrity_hash {
                            return Err(io::Error::other("linked artifact changed"));
                        }
                        queue.push_back(child);
                    }
                }
            };
            Ok(render(
                &selected,
                &format!("/{}", self.token),
                &self.root_id,
            ))
        })();
        match result {
            Ok(body) => ("200 OK", body),
            Err(_) => ("422 Unprocessable Content", error_page()),
        }
    }
}
