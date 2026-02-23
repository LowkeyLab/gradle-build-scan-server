use error::ParseError;

pub struct FramedEvent {
    pub wire_id: u16,
    pub timestamp: i64,
    pub ordinal: i32,
    pub body: Vec<u8>,
}

pub struct EventFrameReader<'a> {
    data: &'a [u8],
    pos: usize,
    wire_id: i64,
    timestamp: i64,
    ordinal: i32,
}

impl<'a> EventFrameReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            wire_id: 0,
            timestamp: 0,
            ordinal: 0,
        }
    }

    fn read_next(&mut self) -> Result<FramedEvent, ParseError> {
        let flags = varint::read_unsigned_varint(self.data, &mut self.pos)? as u8;

        // bit0=0 → type delta present
        if flags & 1 == 0 {
            let delta = varint::read_zigzag_i32(self.data, &mut self.pos)?;
            self.wire_id += delta as i64;
        }
        // bit1=0 → timestamp delta present
        if flags & 2 == 0 {
            let delta = varint::read_zigzag_i64(self.data, &mut self.pos)?;
            self.timestamp += delta;
        }
        // bit2=0 → actual-timestamp delta present (read and discard)
        if flags & 4 == 0 {
            let _actual_delta = varint::read_zigzag_i64(self.data, &mut self.pos)?;
        }
        // bit3=0 → ordinal delta present; bit3=1 → default +1
        if flags & 8 == 0 {
            let delta = varint::read_zigzag_i32(self.data, &mut self.pos)?;
            self.ordinal += delta;
        } else {
            self.ordinal += 1;
        }

        let body_length = varint::read_unsigned_varint(self.data, &mut self.pos)? as usize;
        if self.pos + body_length > self.data.len() {
            return Err(ParseError::UnexpectedEof { offset: self.pos });
        }
        let body = self.data[self.pos..self.pos + body_length].to_vec();
        self.pos += body_length;

        Ok(FramedEvent {
            wire_id: self.wire_id as u16,
            timestamp: self.timestamp,
            ordinal: self.ordinal,
            body,
        })
    }
}

impl<'a> Iterator for EventFrameReader<'a> {
    type Item = Result<FramedEvent, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }
        Some(self.read_next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_five_events_bytes() -> Vec<u8> {
        hex::decode(
            "0c9204b4c584d390670e00bba9c2a9c83301018097930500\
             0e8504000e010802010a6275696c64\
             0efe070301bd10\
             0eef070110",
        )
        .unwrap()
    }

    #[test]
    fn test_first_event_wire_id() {
        let data = first_five_events_bytes();
        let mut reader = EventFrameReader::new(&data);
        let event = reader.next().unwrap().unwrap();
        assert_eq!(event.wire_id, 265); // DAEMON_STATE_v1
        assert_eq!(event.body.len(), 14);
    }

    #[test]
    fn test_five_events_wire_ids() {
        let data = first_five_events_bytes();
        let events: Vec<_> = EventFrameReader::new(&data)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].wire_id, 265); // DAEMON_STATE_v1
        assert_eq!(events[1].wire_id, 6); // BUILD_STARTED
        assert_eq!(events[2].wire_id, 5); // BUILD_REQUESTED_TASKS
        assert_eq!(events[3].wire_id, 516); // BUILD_MODES_v2
        assert_eq!(events[4].wire_id, 12); // HARDWARE
    }

    #[test]
    fn test_ordinals_increment() {
        let data = first_five_events_bytes();
        let events: Vec<_> = EventFrameReader::new(&data)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(events[0].ordinal, 1);
        assert_eq!(events[1].ordinal, 2);
        assert_eq!(events[2].ordinal, 3);
    }

    #[test]
    fn test_body_content() {
        let data = first_five_events_bytes();
        let events: Vec<_> = EventFrameReader::new(&data)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(events[1].body.is_empty());
        assert_eq!(events[2].body.len(), 8);
    }
}
