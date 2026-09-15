//! Bounded stdio MCP server; only trusted, startup-pinned configurations execute.
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Read, Write},
    path::Path,
};
use verify_cli::{
    agent::{self, Operation, Product, Request, Response},
    integration::{self, Prepared, Project},
};
use verify_core::Verdict;
const VERSION: &str = "2025-11-25";
const TOOLS: [&str; 5] = [
    "b2ige_doctor",
    "b2ige_behavior_verify",
    "b2ige_sideeffect_verify",
    "b2ige_blindtest_verify",
    "b2ige_report",
];
pub struct Server {
    initialized: bool,
    ready: bool,
    configs: BTreeMap<String, (Product, Option<Prepared>)>,
    results: BTreeMap<String, (String, String)>,
}
impl Server {
    pub fn open(path: &Path) -> io::Result<Self> {
        let p: Project = integration::read(path)?;
        if p.schema_version != "1" {
            return Err(io::Error::other("unsupported registry"));
        }
        let configs = p
            .entries
            .into_iter()
            .map(|(key, e)| (key, (e.product, Prepared::open(&e).ok())))
            .collect();
        Ok(Self {
            initialized: false,
            ready: false,
            configs,
            results: BTreeMap::new(),
        })
    }
    fn call(&mut self, name: &str, r: Request) -> Response {
        let error = || Response::error(r.product, r.operation);
        if r.execution_budget.is_some() {
            return error();
        }
        let expected = match name {
            "b2ige_doctor" => Operation::Doctor,
            "b2ige_report" => Operation::Report,
            _ => Operation::Verify,
        };
        if expected != r.operation
            || match name {
                "b2ige_behavior_verify" => r.product != Product::Behavior,
                "b2ige_sideeffect_verify" => r.product != Product::Sideeffect,
                "b2ige_blindtest_verify" => r.product != Product::Blindtest,
                _ => false,
            }
        {
            return error();
        }
        if r.operation == Operation::Report {
            let Some((key, id)) = self.results.get(&r.identity) else {
                return error();
            };
            let Some((product, Some(p))) = self.configs.get(key) else {
                return error();
            };
            if *product != r.product {
                return error();
            }
            return p
                .report(id)
                .map(|v| Response::from_verified(&v, Operation::Report))
                .unwrap_or_else(|_| error());
        }
        let Some((product, Some(p))) = self.configs.get(&r.identity) else {
            return error();
        };
        if *product != r.product {
            return error();
        }
        if r.operation == Operation::Doctor {
            return p.doctor(r.product);
        }
        match p.execute() {
            Ok((id, v)) => {
                let response = Response::from_verified(&v, Operation::Verify);
                if response.product != r.product {
                    return error();
                }
                self.results.insert(
                    response
                        .source
                        .as_ref()
                        .expect("verified source")
                        .artifact_id
                        .clone(),
                    (r.identity, id),
                );
                response
            }
            Err(_) => error(),
        }
    }
    pub fn handle(&mut self, v: Value) -> Option<Value> {
        let id = v.get("id").cloned().unwrap_or(Value::Null);
        let err = |code, message| json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}});
        if !v.is_object()
            || v["jsonrpc"] != "2.0"
            || !v["method"].is_string()
            || (v.get("id").is_some() && !id.is_string() && !id.is_i64() && !id.is_u64())
        {
            return Some(err(-32600, "Invalid request"));
        }
        let method = v["method"].as_str().expect("method");
        if v.get("id").is_none() {
            if method == "notifications/initialized" && self.initialized {
                self.ready = true;
            }
            return None;
        }
        let result = match method {
            "initialize" if !self.initialized => {
                if !v["params"]["protocolVersion"].is_string()
                    || !v["params"]["capabilities"].is_object()
                    || !v["params"]["clientInfo"].is_object()
                {
                    return Some(err(-32602, "Invalid initialization"));
                }
                self.initialized = true;
                json!({"protocolVersion":VERSION,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"b2ige-verify","version":env!("CARGO_PKG_VERSION")},"instructions":"Only registered typed configurations execute. PASS is bounded; doctor indicates readiness only. Non-PASS never establishes completion."})
            }
            "ping" => json!({}),
            _ if !self.ready => return Some(err(-32002, "Server not initialized")),
            "tools/list" => {
                json!({"tools":TOOLS.iter().map(|name|json!({"name":name,"description":"Invoke an existing authoritative verifier using a trusted registered identity; sanitized Agent Protocol v1 only","inputSchema":agent::request_schema(),"outputSchema":agent::response_schema()})).collect::<Vec<_>>()})
            }
            "tools/call" => {
                let Some(name) = v["params"]["name"].as_str().filter(|n| TOOLS.contains(n)) else {
                    return Some(err(-32602, "Unknown tool"));
                };
                let request: Request =
                    match serde_json::from_value(v["params"]["arguments"].clone()) {
                        Ok(r) => r,
                        Err(_) => return Some(err(-32602, "Invalid typed tool arguments")),
                    };
                let r = self.call(name, request);
                json!({"content":[{"type":"text","text":serde_json::to_string(&r).expect("response")}],"structuredContent":r,"isError":r.verdict!=Verdict::Pass})
            }
            _ => return Some(err(-32601, "Method not found")),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
    pub fn serve(&mut self, mut input: impl BufRead, mut output: impl Write) -> io::Result<()> {
        loop {
            let mut line = Vec::new();
            let n = Read::take(&mut input, 1024 * 1024 + 1).read_until(b'\n', &mut line)?;
            if n == 0 {
                break;
            }
            if n > 1024 * 1024 {
                return Err(io::Error::other("request too large"));
            }
            let result = match serde_json::from_slice(&line) {
                Ok(v) => self.handle(v),
                Err(_) => Some(
                    json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}),
                ),
            };
            if let Some(r) = result {
                serde_json::to_writer(&mut output, &r)?;
                output.write_all(b"\n")?;
                output.flush()?;
            }
        }
        Ok(())
    }
}
