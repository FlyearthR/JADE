//! Shared types, FFI bindings, and IPC message definitions for the Network Time Simulator.
//!
//! This crate provides the data structures used for communication between the
//! Rust leader (simulator) and the C intercepted applications, as well as
//! common networking types like IP addresses compatible with C representations.

mod follower;

use std::str::FromStr;

/// The fixed size of the buffer used for IPC messages via Posix Message Queues.
pub const SIZE_BUFFER: usize = 27;

/// A C-compatible representation of an IPv4 address.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Ipv4AddrC {
    /// The four octets of the IPv4 address.
    pub segments: [u8; 4],
}

impl Ipv4AddrC {
    /// Creates a new `Ipv4AddrC` from four octets.
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            segments: [a, b, c, d],
        }
    }

    /// Returns the octets of the IPv4 address.
    pub fn octets(self) -> [u8; 4] {
        self.segments
    }
}
impl From<std::net::Ipv4Addr> for Ipv4AddrC {
    fn from(ip: std::net::Ipv4Addr) -> Self {
        let ip_array = ip.octets();
        Self::new(ip_array[0], ip_array[1], ip_array[2], ip_array[3])
    }
}
impl Into<std::net::Ipv4Addr> for Ipv4AddrC {
    fn into(self) -> std::net::Ipv4Addr {
        let ip_array = self.octets();
        std::net::Ipv4Addr::new(ip_array[0], ip_array[1], ip_array[2], ip_array[3])
    }
}
impl From<&str> for Ipv4AddrC {
    fn from(ip: &str) -> Self {
        std::net::Ipv4Addr::from_str(ip).unwrap().into()
    }
}
impl From<String> for Ipv4AddrC {
    fn from(ip: String) -> Self {
        ip.as_str().into()
    }
}
impl From<&String> for Ipv4AddrC {
    fn from(ip: &String) -> Self {
        ip.as_str().into()
    }
}

/// A C-compatible representation of an IPv6 address.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Ipv6AddrC {
    /// The eight 16-bit segments of the IPv6 address.
    pub segments: [u16; 8],
}

impl Ipv6AddrC {
    /// Creates a new `Ipv6AddrC` from eight 16-bit segments.
    /// This function is exported to C.
    #[no_mangle]
    pub extern "C" fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        Self {
            segments: [a, b, c, d, e, f, g, h],
        }
    }

    /// Returns the segments of the IPv6 address.
    pub fn segments(self) -> [u16; 8] {
        self.segments
    }
}
impl From<std::net::Ipv6Addr> for Ipv6AddrC {
    fn from(ip: std::net::Ipv6Addr) -> Self {
        let ip_array = ip.segments();
        Self::new(
            ip_array[0],
            ip_array[1],
            ip_array[2],
            ip_array[3],
            ip_array[4],
            ip_array[5],
            ip_array[6],
            ip_array[7],
        )
    }
}

impl Into<std::net::Ipv6Addr> for Ipv6AddrC {
    fn into(self) -> std::net::Ipv6Addr {
        let ip_array = self.segments();
        std::net::Ipv6Addr::new(
            ip_array[0],
            ip_array[1],
            ip_array[2],
            ip_array[3],
            ip_array[4],
            ip_array[5],
            ip_array[6],
            ip_array[7],
        )
    }
}
impl From<&str> for Ipv6AddrC {
    fn from(ip: &str) -> Self {
        std::net::Ipv6Addr::from_str(ip).unwrap().into()
    }
}
impl From<String> for Ipv6AddrC {
    fn from(ip: String) -> Self {
        ip.as_str().into()
    }
}
impl From<&String> for Ipv6AddrC {
    fn from(ip: &String) -> Self {
        ip.as_str().into()
    }
}
/// Converts a C string to an `Ipv6AddrC`.
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
#[no_mangle]
extern "C" fn ip6_from_str(ip: *const std::ffi::c_char) -> Ipv6AddrC {
    unsafe {
        std::ffi::CStr::from_ptr(ip)
            .to_str()
            .expect("IPv6 could not be parsed to rust string")
            .into()
    }
}

/// Converts a C string to an `Ipv4AddrC`.
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
#[no_mangle]
extern "C" fn ip4_from_str(ip: *const std::ffi::c_char) -> Ipv4AddrC {
    unsafe {
        std::ffi::CStr::from_ptr(ip)
            .to_str()
            .expect("IPv4 could not be parsed to rust string")
            .into()
    }
}

/// A fixed-size buffer for IPC messages.
#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct Buffer {
    pub buffer: [u8; SIZE_BUFFER],
}

impl Buffer {
    /// Creates a new empty `Buffer`.
    pub fn new() -> Buffer {
        Buffer {
            buffer: [0; SIZE_BUFFER],
        }
    }
}

/// Messages exchanged between the simulator (leader) and the intercepted apps (followers).
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Message {
    /// App is blocked on an operation.
    Stuck(u8),
    /// App is requesting to advance time or perform a step.
    AddStep(u8, u64),
    /// App is cancelling a previously requested step.
    DelStep(u8, u64),
    /// App has an IPv4 packet to send.
    HasToSend4(u8, u8, Ipv4AddrC, u64),
    /// App has an IPv6 packet to send.
    HasToSend6(u8, u8, Ipv6AddrC, u64),
    /// Simulator authorizing an app to send a packet.
    Send(u64),
    /// App confirming a packet has been sent.
    Sent(u8, u64),
    /// App requesting the current simulation time.
    GetTime(u8),
    /// App requesting a random seed from the simulator.
    GetRand(u8, u64),
    /// Simulator waking up an app at a specific time.
    WakeUp(u64),
    /// App has finished its execution.
    Finished(u8),
}

impl From<Message> for Buffer {
    fn from(msg: Message) -> Self {
        let mut tab: [u8; SIZE_BUFFER] = [0; SIZE_BUFFER];
        match msg {
            Message::Stuck(id) => {
                tab[0] = 1;
                tab[1] = id;
            }
            Message::AddStep(id, t) => {
                tab[0] = 2;
                tab[1] = id;
                let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i + 2] = tt[i];
                    i = i + 1;
                }
            }
            Message::DelStep(id, t) => {
                tab[0] = 3;
                tab[1] = id;
                let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i + 2] = tt[i];
                    i = i + 1;
                }
            }
            Message::HasToSend4(id, if_id, addr, pkt_id) => {
                tab[0] = 4;
                tab[1] = id;
                tab[2] = if_id;
                let t = addr.octets();
                let mut i = 0;
                while i < 4 {
                    tab[i + 3] = t[i];
                    i = i + 1;
                }
                let t1 = pkt_id.to_ne_bytes();
                i = 0;
                while i < 8 {
                    tab[i + 7] = t1[i];
                    i = i + 1;
                }
            }
            Message::HasToSend6(id, if_id, addr, pkt_id) => {
                tab[0] = 5;
                tab[1] = id;
                tab[2] = if_id;
                let t = addr.segments();
                let mut i = 0;
                while i < 8 {
                    let tt = t[i].to_ne_bytes();
                    tab[2 * i + 3] = tt[0];
                    tab[2 * i + 4] = tt[1];
                    i = i + 1;
                }
                let t1 = pkt_id.to_ne_bytes();
                i = 0;
                while i < 8 {
                    tab[i + 19] = t1[i];
                    i = i + 1;
                }
            }
            Message::Send(pkt_id) => {
                tab[0] = 6;
                let tt = pkt_id.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i + 2] = tt[i];
                    i = i + 1;
                }
            }
            Message::Sent(id, pkt_id) => {
                tab[0] = 7;
                tab[1] = id;
                let tt = pkt_id.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i + 2] = tt[i];
                    i = i + 1;
                }
            }
            Message::GetTime(id) => {
                tab[0] = 8;
                tab[1] = id;
            }
            Message::GetRand(id, seed) => {
                tab[0] = 9;
                tab[1] = id;
                let tt = seed.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i + 2] = tt[i];
                    i = i + 1;
                }
            }
            Message::WakeUp(t) => {
                tab[0] = 10;
                let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i + 2] = tt[i];
                    i = i + 1;
                }
            }
            Message::Finished(id) => {
                tab[0] = 11;
                tab[1] = id;
            }
        }
        Buffer { buffer: tab }
    }
}

impl Into<Message> for Buffer {
    fn into(self) -> Message {
        match self.buffer[0] {
            1 => Message::Stuck(self.buffer[1]),
            2 => Message::AddStep(
                self.buffer[1],
                u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()),
            ),
            3 => Message::DelStep(
                self.buffer[1],
                u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()),
            ),
            4 => Message::HasToSend4(
                self.buffer[1],
                self.buffer[2],
                Ipv4AddrC::new(
                    self.buffer[3],
                    self.buffer[4],
                    self.buffer[5],
                    self.buffer[6],
                ),
                u64::from_ne_bytes(self.buffer[7..15].try_into().unwrap()),
            ),
            5 => Message::HasToSend6(
                self.buffer[1],
                self.buffer[2],
                Ipv6AddrC::new(
                    u16::from_ne_bytes(self.buffer[3..5].try_into().unwrap()),
                    u16::from_ne_bytes(self.buffer[5..7].try_into().unwrap()),
                    u16::from_ne_bytes(self.buffer[7..9].try_into().unwrap()),
                    u16::from_ne_bytes(self.buffer[9..11].try_into().unwrap()),
                    u16::from_ne_bytes(self.buffer[11..13].try_into().unwrap()),
                    u16::from_ne_bytes(self.buffer[13..15].try_into().unwrap()),
                    u16::from_ne_bytes(self.buffer[15..17].try_into().unwrap()),
                    u16::from_ne_bytes(self.buffer[17..19].try_into().unwrap()),
                ),
                u64::from_ne_bytes(self.buffer[19..27].try_into().unwrap()),
            ),
            6 => Message::Send(u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap())),
            7 => Message::Sent(
                self.buffer[1],
                u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()),
            ),
            8 => Message::GetTime(self.buffer[1]),
            9 => Message::GetRand(
                self.buffer[1],
                u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()),
            ),
            10 => Message::WakeUp(u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap())),
            _ => Message::Finished(self.buffer[1]),
        }
    }
}

/// Serializes a `Message` into a `Buffer` for Rust usage.
pub fn serialize_rust(msg: Message) -> Buffer {
    Buffer::from(msg)
}

/// Deserializes a `Buffer` into a `Message` for Rust usage.
pub fn deserialize_rust(msg: Buffer) -> Message {
    Buffer::into(msg)
}

/// Serializes a `Message` and returns a pointer to a `Buffer`.
/// The caller is responsible for freeing the memory.
#[no_mangle]
pub extern "C" fn serialize(msg: Message) -> *mut Buffer {
    Box::into_raw(Box::new(Buffer::from(msg)))
}

/// Deserializes a `Buffer` and returns a pointer to a `Message`.
/// The caller is responsible for freeing the memory.
#[no_mangle]
pub extern "C" fn deserialize(msg: Buffer) -> *mut Message {
    Box::into_raw(Box::new(Buffer::into(msg)))
}
