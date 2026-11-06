//! The link probe: a raw TCP bulk transfer and a ping-pong, which measure
//! a link's goodput and round trip without Rumors.
//!
//! Calibration uses it to judge the emulator itself: whether a shaped link
//! delivers its configured rate and round trip, alone and beside other
//! sessions, independently of the library and of this tool's fixtures. In
//! pair mode it characterizes the network between the hosts before the run.
//!
//! The conversation with a probe process follows [`crate::peer`]'s form:
//! side A receives [`ProbeCommand::Listen`], answers
//! [`ProbeReply::Listening`], and serves one probe connection; side B
//! receives [`ProbeCommand::Run`], runs the probe against it, and answers
//! [`ProbeReply::Probed`]. On the connection, B sends an eight-byte length
//! and that many bytes; A answers one byte once it has read them all; then
//! B sends one byte at a time, each echoed by A, for the ping-pong.

use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::peer::{receive, send};

/// A driver's instruction to a probe process.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ProbeCommand {
    /// Side A: listen on this address and serve one probe.
    Listen {
        /// The address to bind, port 0 for a free port.
        address: SocketAddr,
    },
    /// Side B: probe side A at this address.
    Run {
        /// Side A's address.
        address: SocketAddr,
        /// Bytes in the bulk transfer.
        bulk_bytes: u64,
        /// Round trips in the ping-pong.
        pings: u32,
    },
}

/// A probe process's answer.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "snake_case")]
pub enum ProbeReply {
    /// Side A listens on this port.
    Listening {
        /// The bound port.
        port: u16,
    },
    /// Side A served its probe.
    Served,
    /// Side B's measurements.
    Probed {
        /// The measurements.
        result: ProbeResult,
    },
}

/// What one probe measured.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ProbeResult {
    /// The median ping-pong round trip, in seconds.
    pub rtt_seconds: f64,
    /// Seconds from the first bulk byte written to the acknowledgement read.
    pub transfer_seconds: f64,
    /// Bulk bytes over the transfer time less one round trip (the first
    /// byte's way there and the acknowledgement's way back): the rate the
    /// link delivered, slow start included.
    pub goodput_bytes_per_second: f64,
}

/// Runs one probe process to completion over a line-oriented control
/// channel.
///
/// # Errors
///
/// If the control channel or the probe connection fails.
pub fn serve_one(control_in: &mut impl BufRead, control_out: &mut impl Write) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("starting the probe's runtime")?;
    runtime.block_on(async {
        match receive(control_in)? {
            ProbeCommand::Listen { address } => {
                let listener = TcpListener::bind(address)
                    .await
                    .with_context(|| format!("binding {address}"))?;
                send(
                    control_out,
                    &ProbeReply::Listening {
                        port: listener.local_addr()?.port(),
                    },
                )?;
                let (stream, _) = listener.accept().await?;
                answer(stream).await?;
                send(control_out, &ProbeReply::Served)
            }
            ProbeCommand::Run {
                address,
                bulk_bytes,
                pings,
            } => {
                let stream = TcpStream::connect(address)
                    .await
                    .with_context(|| format!("connecting to the probe at {address}"))?;
                let result = probe(stream, bulk_bytes, pings).await?;
                send(control_out, &ProbeReply::Probed { result })
            }
        }
    })
}

/// The buffer the bulk transfer writes from and reads into.
const CHUNK: usize = 1 << 16;

/// Side A of a probe connection.
async fn answer(mut stream: TcpStream) -> Result<()> {
    stream.set_nodelay(true)?;
    let mut length = [0u8; 8];
    stream.read_exact(&mut length).await?;
    let mut left = u64::from_le_bytes(length);
    let mut sink = vec![0u8; CHUNK];
    while left > 0 {
        let n = stream
            .read(&mut sink[..left.min(CHUNK as u64) as usize])
            .await?;
        if n == 0 {
            bail!("the probe connection closed during the bulk transfer");
        }
        left -= n as u64;
    }
    stream.write_all(&[1]).await?;
    let mut byte = [0u8; 1];
    while stream.read(&mut byte).await? == 1 {
        stream.write_all(&byte).await?;
    }
    Ok(())
}

/// Side B of a probe connection.
async fn probe(mut stream: TcpStream, bulk_bytes: u64, pings: u32) -> Result<ProbeResult> {
    stream.set_nodelay(true)?;
    let chunk = vec![0u8; CHUNK];
    let began = Instant::now();
    stream.write_all(&bulk_bytes.to_le_bytes()).await?;
    let mut left = bulk_bytes;
    while left > 0 {
        let n = left.min(CHUNK as u64) as usize;
        stream.write_all(&chunk[..n]).await?;
        left -= n as u64;
    }
    let mut byte = [0u8; 1];
    stream.read_exact(&mut byte).await?;
    let transfer_seconds = began.elapsed().as_secs_f64();
    let mut rtts = Vec::with_capacity(pings as usize);
    for _ in 0..pings {
        let sent = Instant::now();
        stream.write_all(&[7]).await?;
        stream.read_exact(&mut byte).await?;
        rtts.push(sent.elapsed().as_secs_f64());
    }
    stream.shutdown().await?;
    rtts.sort_by(f64::total_cmp);
    let rtt_seconds = rtts.get(rtts.len() / 2).copied().unwrap_or(0.0);
    let moving = (transfer_seconds - rtt_seconds).max(f64::MIN_POSITIVE);
    Ok(ProbeResult {
        rtt_seconds,
        transfer_seconds,
        goodput_bytes_per_second: bulk_bytes as f64 / moving,
    })
}
