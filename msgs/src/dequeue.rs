use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt};


pub fn dequeue_msg(input_buffer: &mut Vec<u8>) -> Option<(usize, usize)> {
    if input_buffer.len() < 4 {
        return None
    }

    let mut rdr = Cursor::new(&input_buffer);

    let msg_ln = rdr.read_u32::<LittleEndian>().unwrap() as usize;

    if msg_ln > 2000 {
        println!("long message: {msg_ln}");
    }

    let end = msg_ln + 4;

    if input_buffer.len() < end {
        return None
    }

    Some((4, end))
}
