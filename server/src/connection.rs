use std::io;
use std::sync::Arc;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use crate::engine::Engine;

pub async fn handle(socket: TcpStream, engine: Arc<Engine>) -> io::Result<()> {
    todo!()
}
