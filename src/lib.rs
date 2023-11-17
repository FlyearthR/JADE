mod follower;

//use serde::{Serialize, Deserialize};

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct Buffer {
    pub buffer: [u8; 10],
}


#[repr(C)]
//#[derive(Serialize, Deserialize)]
pub enum Message {
    Progressed(u8),
    Stuck(u8),
    AddStep(u8, u64),
    DelStep(u8, u64),
    GetTime(u8),
    GetRand(u8),
    WakeUp(u64),
    Finished(u8),
}

impl From<Message> for Buffer {
    fn from(msg: Message) -> Self {
        let mut tab: [u8; 10] = [0; 10];
        match msg {
            Message::Progressed(id) => {
                tab[0] = 0;
                tab[1] = id;
            },
            Message::Stuck(id) => {
                tab[0] = 1;
                tab[1] = id;
            },
            Message::AddStep(id, t) => {
                tab[0] = 2;
                tab[1] = id;
                /*let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i+2] = tt[i];
                    i = i + 1;
                }*/
                let mut tt = t;
                tab[2] = (tt & 0b11111111) as u8;
                tt /= 256;
                tab[3] = (tt & 0b11111111) as u8;
                tt /= 256;
                tab[4] = (tt & 0b11111111) as u8;
                tt /= 256;
                tab[5] = (tt & 0b11111111) as u8;
                tt /= 256;
                tab[6] = (tt & 0b11111111) as u8;
                tt /= 256;
                tab[7] = (tt & 0b11111111) as u8;
                tt /= 256;
                tab[8] = (tt & 0b11111111) as u8;
                tt /= 256;
                tab[9] = (tt & 0b11111111) as u8;
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
            Message::GetTime(id) => {
                tab[0] = 4;
                tab[1] = id;
            },
            Message::GetRand(id) => {
                tab[0] = 5;
                tab[1] = id;
            },
            Message::WakeUp(t) => {
                tab[0] = 6;
                let tt = t.to_ne_bytes();
                let mut i = 0;
                while i < 8 {
                    tab[i+2] = tt[i];
                    i = i + 1;
                }
            },
            Message::Finished(id) => {
                tab[0] = 7;
                tab[1] = id;
            },
        }
        Buffer {buffer : tab}
    }
}

/*impl Into<Vec<u8> for Buffer {
    fn into(self) -> Vec<u8> {
        unsafe {
            Vec::from_raw_parts(self.buffer as *mut u8, self.len as usize, self.len as usize)
        }
    }
}*/

impl Into<Message> for Buffer {
    fn into(self) -> Message {
        match self.buffer[0] {
            0 => {
                Message::Progressed(self.buffer[1])
            },
            1 => {
                Message::Stuck(self.buffer[1])
            },
            2 => {
                let mut tt: u64 = 0;
                tt += self.buffer[9] as u64;
                tt *= 256;
                tt += self.buffer[8] as u64;
                tt *= 256;
                tt += self.buffer[7] as u64;
                tt *= 256;
                tt += self.buffer[6] as u64;
                tt *= 256;
                tt += self.buffer[5] as u64;
                tt *= 256;
                tt += self.buffer[4] as u64;
                tt *= 256;
                tt += self.buffer[3] as u64;
                tt *= 256;
                tt += self.buffer[2] as u64;
                Message::AddStep(self.buffer[1], tt)
            },
            3 => {
                Message::DelStep(self.buffer[1], u64::from_be_bytes(self.buffer[2..10].try_into().unwrap()))
            },
            4 => {
                Message::GetTime(self.buffer[1])
            },
            5 => {
                Message::GetRand(self.buffer[1])
            },
            6 => {
                Message::WakeUp(u64::from_be_bytes(self.buffer[2..10].try_into().unwrap()))
            },
            _ => {
                Message::Finished(self.buffer[1])
            },
        }
    }
}

#[no_mangle]
pub extern "C" fn serialize(msg: Message) -> Buffer {
    //let msg_vec = serde_cbor::to_vec(&msg).expect("Unable to serialize message");
    Buffer::from(msg)
}

#[no_mangle]
pub extern "C" fn deserialize(msg: Buffer) -> Message {
    Buffer::into(msg)
}

/*#[allow(dead_code)]
pub fn deserialize(msg: &[u8]) -> Message {
    serde_cbor::from_slice(msg).unwrap()
}

pub fn serialize_u64(msg: u64) -> [u8; 8] {
    msg.to_be_bytes()
}*/