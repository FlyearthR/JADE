mod follower;

use std::str::FromStr;

pub const SIZE_BUFFER: usize = 27;

#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Ipv4AddrC {
    pub segments: [u8; 4],
}
impl Ipv4AddrC {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self { segments: [a, b, c, d] }
    }

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
    fn into(self)->std::net::Ipv4Addr{
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

#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Ipv6AddrC {
    pub segments: [u16; 8],
}
impl Ipv6AddrC {
    #[no_mangle]
    pub extern "C" fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        Self { segments: [a, b, c, d, e, f, g, h] }
    }

    pub fn segments(self) -> [u16; 8] {
        self.segments
    }
}
impl From<std::net::Ipv6Addr> for Ipv6AddrC {
    fn from(ip: std::net::Ipv6Addr) -> Self {
        let ip_array = ip.segments();
        Self::new(ip_array[0], ip_array[1], ip_array[2], ip_array[3],
            ip_array[4], ip_array[5], ip_array[6], ip_array[7])
    }
}

impl Into<std::net::Ipv6Addr> for Ipv6AddrC {
    fn into(self)->std::net::Ipv6Addr{
        let ip_array = self.segments();
        std::net::Ipv6Addr::new(ip_array[0], ip_array[1], ip_array[2], ip_array[3],
            ip_array[4],ip_array[5],ip_array[6],ip_array[7])
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
#[no_mangle]
extern "C" fn ip6_from_str(ip: *const std::ffi::c_char) -> Ipv6AddrC {
    unsafe {
        std::ffi::CStr::from_ptr(ip).to_str().expect("IPv6 could not be parsed to rust string").into()
    }
}
#[no_mangle]
extern "C" fn ip4_from_str(ip: *const std::ffi::c_char) -> Ipv4AddrC {
    unsafe {
        std::ffi::CStr::from_ptr(ip).to_str().expect("IPv4 could not be parsed to rust string").into()
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct Buffer {
    pub buffer: [u8; SIZE_BUFFER],
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer { buffer: [0; SIZE_BUFFER] }
    }
}


#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Message {
    Stuck(u8),                          // 1
    AddStep(u8, u64),                   // 2
    DelStep(u8, u64),                   // 3
    HasToSend4(u8, u8, Ipv4AddrC, u64), // 4
    HasToSend6(u8, u8, Ipv6AddrC, u64), // 5
    Send(u64),                          // 6
    Sent(u8, u64),                      // 7
    GetTime(u8),                        // 8
    GetRand(u8),                        // 9
    WakeUp(u64),                        // 10
    Finished(u8),                       // 11
}

impl From<Message> for Buffer {
    fn from(msg: Message) -> Self {
        let mut tab: [u8; SIZE_BUFFER] = [0; SIZE_BUFFER];
        match msg {
            Message::Stuck(id) => {
                tab[0] = 1;
                tab[1] = id;
            },
            Message::AddStep(id, t) => {
                tab[0] = 2;
                tab[1] = id;
                let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i+2] = tt[i];
                    i = i + 1;
                }
            },
            Message::DelStep(id, t) => {
                tab[0] = 3;
                tab[1] = id;
                let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i+2] = tt[i];
                    i = i + 1;
                }
            },
            Message::HasToSend4(id, if_id, addr, pkt_id) => {
                tab[0] = 4;
                tab[1] = id;
                tab[2] = if_id;
                let t = addr.octets();
                let mut i = 0;
                while i < 4 {
                    tab[i+3] = t[i];
                    i = i + 1;
                }
                let t1 = pkt_id.to_ne_bytes();
                i = 0;
                while i < 8 {
                    tab[i+7] = t1[i];
                    i = i + 1;
                }
            },
            Message::HasToSend6(id, if_id, addr, pkt_id) => {
                tab[0] = 5;
                tab[1] = id;
                tab[2] = if_id;
                let t = addr.segments();
                let mut i = 0;
                while i < 8 {
                    let tt = t[i].to_ne_bytes();
                    tab[2*i+3] = tt[0];
                    tab[2*i+4] = tt[1];
                    i = i + 1;
                }
                let t1 = pkt_id.to_ne_bytes();
                i = 0;
                while i < 8 {
                    tab[i+19] = t1[i];
                    i = i + 1;
                }
            },
            Message::Send(pkt_id) => {
                tab[0] = 6;
                let tt = pkt_id.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i+2] = tt[i];
                    i = i + 1;
                }
            },
            Message::Sent(id, pkt_id) => {
                tab[0] = 7;
                tab[1] = id;
                let tt = pkt_id.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i+2] = tt[i];
                    i = i + 1;
                }
            },
            Message::GetTime(id) => {
                tab[0] = 8;
                tab[1] = id;
            },
            Message::GetRand(id) => {
                tab[0] = 9;
                tab[1] = id;
            },
            Message::WakeUp(t) => {
                tab[0] = 10;
                let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i+2] = tt[i];
                    i = i + 1;
                }
            },
            Message::Finished(id) => {
                tab[0] = 11;
                tab[1] = id;
            },
        }        
        Buffer {buffer : tab}
    }
}

impl Into<Message> for Buffer {
    fn into(self) -> Message {
        match self.buffer[0] {
            1 => {
                Message::Stuck(self.buffer[1])
            },
            2 => {
                Message::AddStep(self.buffer[1], u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()))
            },
            3 => {
                Message::DelStep(self.buffer[1], u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()))
            },
            4 => {
                Message::HasToSend4(self.buffer[1], self.buffer[2],
                    Ipv4AddrC::new(self.buffer[3], self.buffer[4], self.buffer[5], self.buffer[6]),
                    u64::from_ne_bytes(self.buffer[7..15].try_into().unwrap()))
            },
            5 => {
                Message::HasToSend6(self.buffer[1], self.buffer[2],
                    Ipv6AddrC::new(u16::from_ne_bytes(self.buffer[3..5].try_into().unwrap()), 
                        u16::from_ne_bytes(self.buffer[5..7].try_into().unwrap()), 
                        u16::from_ne_bytes(self.buffer[7..9].try_into().unwrap()), 
                        u16::from_ne_bytes(self.buffer[9..11].try_into().unwrap()),
                        u16::from_ne_bytes(self.buffer[11..13].try_into().unwrap()), 
                        u16::from_ne_bytes(self.buffer[13..15].try_into().unwrap()), 
                        u16::from_ne_bytes(self.buffer[15..17].try_into().unwrap()), 
                        u16::from_ne_bytes(self.buffer[17..19].try_into().unwrap())),
                    u64::from_ne_bytes(self.buffer[19..27].try_into().unwrap()))
            },
            6 => {
                Message::Send(u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()))
            },
            7 => {
                Message::Sent(self.buffer[1], u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()))
            },
            8 => {
                Message::GetTime(self.buffer[1])
            },
            9 => {
                Message::GetRand(self.buffer[1])
            },
            10 => {
                Message::WakeUp(u64::from_ne_bytes(self.buffer[2..10].try_into().unwrap()))
            },
            _ => {
                Message::Finished(self.buffer[1])
            },
        }
    }
}

pub fn serialize_rust(msg: Message) -> Buffer {
    Buffer::from(msg)
}

pub fn deserialize_rust(msg: Buffer) -> Message {
    Buffer::into(msg)
}

#[no_mangle]
pub extern "C" fn serialize(msg: Message) -> *mut Buffer {
    Box::into_raw(Box::new(Buffer::from(msg)))

}

#[no_mangle]
pub extern "C" fn deserialize(msg: Buffer) -> *mut Message {
    Box::into_raw(Box::new(Buffer::into(msg)))
}