use std::{
    io::{BufReader, BufWriter, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{Receiver, Sender, channel},
    thread::JoinHandle,
};

pub struct SubprocessTransport {
    pub child: Child,
    pub send: Sender<Vec<u8>>,
    pub recv: Receiver<Vec<u8>>,
    pub recv_err: Receiver<Vec<u8>>,
    pub threads: [JoinHandle<()>; 3], // TODO join?
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

        let (send_request, recv_request) = channel::<Vec<u8>>();
        let (send_response, recv_response) = channel::<Vec<u8>>();
        let (send_err, recv_err) = channel::<Vec<u8>>();
        let request_thread = std::thread::spawn(move || {
            while let Ok(bytes) = recv_request.recv() {
                stdin.write_all(&bytes).unwrap();
                stdin.flush().unwrap();
            }
        });
        let response_thread = std::thread::spawn(move || {
            let mut buf = vec![0u8; 8192];
            loop {
                let n = stdout.read(&mut buf).unwrap();
                if n != 0 {
                    send_response.send(buf[..n].to_vec()).unwrap();
                }
            }
        });
        let err_thread = std::thread::spawn(move || {
            let mut buf = vec![0u8; 8192];
            loop {
                let n = stderr.read(&mut buf).unwrap();
                if n != 0 {
                    send_err.send(buf[..n].to_vec()).unwrap();
                }
            }
        });

        Self {
            child,
            send: send_request,
            recv: recv_response,
            recv_err,
            threads: [request_thread, response_thread, err_thread],
        }
    }

    pub fn send(&self, bytes: Vec<u8>) {
        self.send.send(bytes).unwrap()
    }

    pub fn recv(&self) -> impl Iterator<Item = Vec<u8>> {
        self.recv.try_iter()
    }
}
