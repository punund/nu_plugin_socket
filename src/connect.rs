use super::SocketPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    ByteStream, ByteStreamSource, ByteStreamType, Category, DataSource,
    Example, LabeledError, PipelineData, PipelineMetadata, Record,
    Signature, SyntaxShape, Value,
};
use std::io::Write;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

pub struct Connect;

impl PluginCommand for Connect {
    type Plugin = SocketPlugin;

    fn name(&self) -> &str {
        "socket connect"
    }

    fn description(&self) -> &str {
        "Connect to a remote host, send data from stdin, and stream the reply to stdout."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "host",
                SyntaxShape::String,
                "The hostname or IP address to connect to.",
            )
            .required("port", SyntaxShape::Int, "The port number to connect to.")
            .named(
                "timeout",
                SyntaxShape::Duration,
                "Timeout for network operations. Defaults to 10 seconds.",
                Some('t'),
            )
            .switch("udp", "Use UDP protocol instead of TCP.", Some('u'))
            .category(Category::Network)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#""GET / HTTP/1.1\r\nHost: example.com\r\n\r\n" | socket connect example.com 80 | decode"#,
                description: "Connect to a web server on port 80 using TCP.",
                result: None,
            },
            Example {
                example: r#""il\r\n" | socket connect whois.iana.org 43"#,
                description: "This command queries a WHOIS server for information about the `.il` domain.",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let head = call.head;
        let host: String = call.req(0)?;
        let port_val: i64 = call.req(1)?;
        let port: u16 = port_val.try_into().map_err(|e| {
            LabeledError::new("Invalid port number")
                .with_help(format!(
                    "Port must be between 0 and 65535. Error: {}",
                    e
                ))
                .with_label("here", call.positional[1].span())
        })?;

        let use_udp = call.has_flag("udp")?;

        let timeout_val: Option<i64> = call.get_flag("timeout")?;
        let timeout = Duration::from_nanos(
            timeout_val.unwrap_or(10_000_000_000) as u64,
        );

        let input_val = input.into_value(head)?;
        let input_bytes = match &input_val {
            Value::String { val, .. } => val.as_bytes().to_vec(),
            Value::Binary { val, .. } => val.clone(),
            Value::Nothing { .. } => vec![],
            other => {
                return Err(LabeledError::new("Unsupported input type")
                    .with_help(format!(
                        "Expected string or binary, but got {}",
                        other.get_type()
                    ))
                    .with_label("input originates from here", head))
            }
        };

        let addr = format!("{}:{}", host, port);
        let socket_addr: SocketAddr = addr
            .to_socket_addrs()
            .map_err(|e| {
                LabeledError::new("Failed to resolve host")
                    .with_help(e.to_string())
                    .with_label(
                        "for this host",
                        call.positional[0].span(),
                    )
            })?
            .next()
            .ok_or_else(|| {
                LabeledError::new("No IP addresses found for host")
                    .with_label(
                        "for this host",
                        call.positional[0].span(),
                    )
            })?;

        if use_udp {
            // --- UDP LOGIC (FIXED) ---
            let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| {
                LabeledError::new("Failed to bind UDP socket")
                    .with_help(e.to_string())
                    .with_label("here", head)
            })?;
            socket.set_read_timeout(Some(timeout)).map_err(|e| {
                LabeledError::new("Failed to set UDP read timeout")
                    .with_help(e.to_string())
                    .with_label("here", head)
            })?;

            // 1. Use `send_to` to send the data to the destination.
            socket.send_to(&input_bytes, socket_addr).map_err(|e| {
                LabeledError::new("Failed to send UDP packet")
                    .with_help(e.to_string())
                    .with_label("here", head)
            })?;

            let mut buffer = vec![0u8; 65535];

            // 2. Use `recv_from` to get the reply from ANY source IP.
            let (bytes_read, _source_addr) =
                socket.recv_from(&mut buffer).map_err(|e| {
                    LabeledError::new(
                        "Failed to receive UDP packet (timed out?)",
                    )
                    .with_help(e.to_string())
                    .with_label("here", head)
                })?;

            buffer.truncate(bytes_read);

            Ok(PipelineData::Value(Value::binary(buffer, head), None))
        } else {
            // --- TCP LOGIC (unchanged) ---
            let mut stream =
                TcpStream::connect_timeout(&socket_addr, timeout)
                    .map_err(|e| {
                        LabeledError::new(
                            "Connection timed out or failed",
                        )
                        .with_help(e.to_string())
                        .with_label("here", head)
                    })?;
            stream.set_read_timeout(Some(timeout)).map_err(|e| {
                LabeledError::new("Failed to set read timeout")
                    .with_help(e.to_string())
                    .with_label("here", head)
            })?;

            stream.write_all(&input_bytes).map_err(|e| {
                LabeledError::new("Failed to write to socket")
                    .with_help(e.to_string())
                    .with_label("here", head)
            })?;

            let source = ByteStreamSource::Read(Box::new(stream));
            let signals = engine.signals().clone();
            let byte_stream = ByteStream::new(
                source,
                head,
                signals,
                ByteStreamType::Unknown,
            );

            let metadata = Some(PipelineMetadata {
                data_source: DataSource::None,
                content_type: None,
                custom: Record::new(),
            });

            Ok(PipelineData::ByteStream(byte_stream, metadata))
        }
    }
}
