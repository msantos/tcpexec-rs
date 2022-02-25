use std::env;
use std::ffi::CString;
use std::fmt::Write;
use std::net::{SocketAddr, TcpListener};
use std::os::unix::io::IntoRawFd;
use std::process::exit;

use socket2::{Domain, Socket, Type};

use nix::unistd::{dup2, execvp};

const PROGNAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

const STDIN_FILENO: i32 = 0;
const STDOUT_FILENO: i32 = 1;
const SOMAXCONN: i32 = 128;

fn usage() -> ! {
    eprintln!(
        r#"{} {}
usage: <IPADDR>:<PORT> <COMMAND> <...>"#,
        PROGNAME, VERSION,
    );
    exit(1);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() < 2 {
        usage()
    }

    let argv: Vec<_> = args[1..]
        .iter()
        .map(|arg| CString::new(arg.as_str()).unwrap())
        .collect();

    let laddr = args[0].to_owned().parse()?;
    let lsock = socket(laddr)?;

    let listener: TcpListener = lsock.into();

    let (stream, raddr) = listener.accept()?;
    let fd = stream.into_raw_fd();

    setremoteenv(raddr)?;
    setlocalenv(laddr)?;

    dup2(fd, STDIN_FILENO)?;
    dup2(fd, STDOUT_FILENO)?;

    execvp(&argv[0], &argv)?;

    unreachable!()
}

fn socket(addr: SocketAddr) -> std::io::Result<Socket> {
    let s = Socket::new_raw(Domain::IPV6, Type::STREAM, None)?;

    s.set_only_v6(false)?;
    s.set_reuse_address(true)?;
    s.set_reuse_port(true)?;
    s.bind(&addr.into())?;
    s.listen(SOMAXCONN)?;

    Ok(s)
}

fn setremoteenv(addr: SocketAddr) -> Result<(), std::fmt::Error> {
    let mut raddr = String::new();
    let mut rport = String::new();

    write!(&mut raddr, "{}", addr.ip())?;
    write!(&mut rport, "{}", addr.port())?;

    env::remove_var("TCPREMOTEHOST");
    env::remove_var("TCPREMOTEINFO");

    env::set_var("PROTO", "TCP");
    env::set_var("TCPREMOTEIP", raddr);
    env::set_var("TCPREMOTEPORT", rport);

    Ok(())
}

fn setlocalenv(addr: SocketAddr) -> Result<(), std::fmt::Error> {
    let mut laddr = String::new();
    let mut lport = String::new();

    write!(&mut laddr, "{}", addr.ip())?;
    write!(&mut lport, "{}", addr.port())?;

    env::remove_var("TCPLOCALHOST");

    env::set_var("TCPLOCALIP", laddr);
    env::set_var("TCPLOCALPORT", lport);

    Ok(())
}
