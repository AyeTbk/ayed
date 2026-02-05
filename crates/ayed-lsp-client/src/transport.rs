use std::{
    io::{BufReader, BufWriter, Read, Write},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    thread::JoinHandle,
};

pub struct SubprocessTransport {
    #[expect(dead_code)]
    pub child: Child,
    pub send_client_msg: Sender<Vec<u8>>,
    pub recv_server_msg: Receiver<Vec<u8>>,
    pub recv_server_err: Receiver<Vec<u8>>,
    pub shutdown_flag: Arc<AtomicBool>,
    pub threads: [JoinHandle<()>; 3],
}

impl SubprocessTransport {
    pub fn new(command: &str) -> Self {
        let mut child = Command::new(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = BufWriter::new(child.stdin.take().unwrap());
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut stderr = BufReader::new(child.stderr.take().unwrap());

        let (send_client_msg, recv_client_msg) = channel::<Vec<u8>>();
        let (send_server_msg, recv_server_msg) = channel::<Vec<u8>>();
        let (send_server_err, recv_server_err) = channel::<Vec<u8>>();

        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let send_to_server_thread = std::thread::spawn({
            let shutdown_flag = shutdown_flag.clone();
            move || {
                while let Ok(bytes) = recv_client_msg.recv() {
                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    stdin.write_all(&bytes).unwrap();
                    stdin.flush().unwrap();
                }
            }
        });

        let recv_from_server_thread = std::thread::spawn({
            let shutdown_flag = shutdown_flag.clone();
            move || {
                loop {
                    let should_shutdown = shutdown_flag.load(Ordering::Relaxed);
                    let len;
                    match read_header(&mut stdout) {
                        Ok(header_len) => len = header_len,
                        Err(ParseHeaderError::EndOfFile) => break,
                        Err(err) => {
                            if should_shutdown {
                                break;
                            } else {
                                panic!("{err:?}");
                            }
                        }
                    };
                    let mut content = vec![0u8; len as usize];
                    stdout.read_exact(&mut content[..]).unwrap();
                    send_server_msg.send(content).unwrap();
                }
            }
        });

        let recv_server_err_thread = std::thread::spawn({
            let shutdown_flag = shutdown_flag.clone();
            move || {
                let mut buf = vec![0u8; 8192];
                loop {
                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let n = stderr.read(&mut buf).unwrap();
                    if n != 0 {
                        send_server_err.send(buf[..n].to_vec()).unwrap();
                    }
                }
            }
        });

        Self {
            child,
            send_client_msg,
            recv_server_msg,
            recv_server_err,
            shutdown_flag,
            threads: [
                send_to_server_thread,
                recv_from_server_thread,
                recv_server_err_thread,
            ],
        }
    }

    pub fn send(&self, bytes: Vec<u8>) {
        self.send_client_msg.send(bytes).unwrap()
    }

    pub fn recv(&self) -> impl Iterator<Item = Vec<u8>> {
        self.recv_server_msg.try_iter()
    }

    pub fn shutdown(self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        self.send_client_msg.send(Default::default()).unwrap();
        for handle in self.threads {
            handle.join().unwrap();
        }
    }
}

fn read_header(r: &mut impl Read) -> Result<u64, ParseHeaderError> {
    read_bstr(r, b"Content-Length: ")?;

    let mut content_len: u64 = 0;
    let mut b;
    loop {
        b = read_byte(r)?;
        if b.is_ascii_digit() {
            let digit = (b - b'0') as u64;
            content_len *= 10;
            content_len += digit;
        } else {
            break;
        }
    }

    if b != b'\r' || read_byte(r)? != b'\n' {
        return Err(ParseHeaderError::BadFormat);
    }
    read_bstr(r, b"\r\n")?;

    Ok(content_len)
}

fn read_bstr(r: &mut impl Read, bstr: &[u8]) -> Result<(), ParseHeaderError> {
    for &bc in bstr {
        let byte = read_byte(r)?;
        if byte != bc {
            return Err(ParseHeaderError::BadFormat);
        }
    }
    Ok(())
}

fn read_byte(r: &mut impl Read) -> Result<u8, ParseHeaderError> {
    let mut buf = [0u8];
    r.read_exact(&mut buf)
        .map_err(|_| ParseHeaderError::EndOfFile)?;
    Ok(buf[0])
}

#[derive(Debug, PartialEq)]
enum ParseHeaderError {
    EndOfFile,
    BadFormat,
}
