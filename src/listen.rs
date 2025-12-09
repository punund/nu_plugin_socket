use super::SocketPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    engine::Closure, Category, Example, LabeledError, PipelineData,
    ShellError, Signature, Spanned, SyntaxShape, Value,
};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

pub struct Listen;

impl PluginCommand for Listen {
    type Plugin = SocketPlugin;

    fn name(&self) -> &str {
        "socket listen"
    }
    fn description(&self) -> &str {
        "Listen for incoming connections and run a closure for each request."
    }
    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("host", SyntaxShape::String, "The hostname or IP address to listen on.")
            .required("port", SyntaxShape::Int, "The port to listen on.")
            .required( "closure", SyntaxShape::Closure(Some(vec![SyntaxShape::Binary])), "The closure to run for each connection. It receives the request as binary.")
                        .switch("single", "Terminate the server after handling a single connection.", Some('s'))

            .category(Category::Network)
    }
    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: r#"socket listen 0.0.0.0 8080 { |request| "Hello, you sent: " ++ ($request | decode) }"#,
            description: "Start a simple echo server on port 8080.",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let head = call.head;
        let host: String = call.req(0)?;
        let port: i64 = call.req(1)?;
        let closure: Closure = call.req(2)?;
        let is_single_shot = call.has_flag("single")?;

        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).map_err(|e| {
            LabeledError::new("Failed to bind to address")
                .with_help(e.to_string())
                .with_label("here", head)
        })?;

        // Set the listener to non-blocking mode.
        listener.set_nonblocking(true).map_err(|e| {
            LabeledError::new("Failed to set listener to non-blocking")
                .with_help(e.to_string())
                .with_label("here", head)
        })?;

        eprintln!("Listening on {}... (Press Ctrl+C to stop)", addr);

        loop {
            // 1. Check for the signal at the beginning of every single loop iteration.
            if engine.signals().interrupted() {
                eprintln!("\nServer shutting down.");
                break;
            }

            // 2. Try to accept a connection.
            match listener.accept() {
                Ok((stream, _addr)) => {
                    // A client connected! Handle it in a new thread like before.
                    let engine = engine.clone();
                    let closure = closure.clone();
                    let head = head;

                    thread::spawn(move || {
                        if let Err(e) = handle_connection(
                            engine, stream, closure, head,
                        ) {
                            eprintln!(
                                "Error in connection handler: {:?}",
                                e
                            );
                        }
                    });
                    if is_single_shot {
                        break;
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // `WouldBlock` means no client is waiting.
                    // This is our normal "idle" state. We sleep briefly to avoid
                    // consuming 100% of the CPU in a tight loop.
                    thread::sleep(Duration::from_millis(50));
                    continue; // Go to the next loop iteration to check for Ctrl-C again.
                }
                Err(e) => {
                    // A real error occurred.
                    eprintln!("Error accepting connection: {}", e);
                    break;
                }
            }
        }

        Ok(PipelineData::empty())
    }
}

fn handle_connection(
    engine: EngineInterface,
    mut stream: TcpStream,
    closure: Closure,
    head: nu_protocol::Span,
) -> Result<(), ShellError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| ShellError::GenericError {
            error: "Failed to set read timeout".into(),
            msg: e.to_string(),
            span: Some(head),
            help: None,
            inner: vec![],
        })?;
    let mut request_bytes = vec![0; 4096];
    let bytes_read = stream.read(&mut request_bytes).map_err(|e| ShellError::GenericError {
        error: "Failed to read from socket".into(), msg: e.to_string(), span: Some(head),
        help: Some("This can happen if the client disconnects or the read times out.".into()), inner: vec![]
    })?;
    request_bytes.truncate(bytes_read);

    let positional_arg = Value::binary(request_bytes, head);
    let positional_args = vec![positional_arg];
    let pipeline_input = None;
    let spanned_closure = Spanned {
        item: closure,
        span: head,
    };
    let response_value = engine.eval_closure(
        &spanned_closure,
        positional_args,
        pipeline_input,
    )?;

    let response_bytes = match response_value {
        Value::String { val, .. } => val.into_bytes(),
        Value::Binary { val, .. } => val,
        other => return Err(ShellError::GenericError {
            error: "Unsupported closure output".into(),
            msg: format!("Expected string or binary from closure, but got {}.", other.get_type()),
            span: Some(head),
            help: Some("The closure for `socket listen` must return a string or binary value.".into()),
            inner: vec![],
        })
    };

    stream.write_all(&response_bytes).map_err(|e| {
        ShellError::GenericError {
            error: "Failed to write to socket".into(),
            msg: e.to_string(),
            span: Some(head),
            help: None,
            inner: vec![],
        }
    })?;

    Ok(())
}
