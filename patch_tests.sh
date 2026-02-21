sed -i '/fn test_next_string()/,/fn test_next_varint/c\
    #[test]\
    fn test_explicit_string() {\
        let mut data = vec![0x0A]; // length 5, shifted 1 = 10 (0x0A)\
        data.extend_from_slice(b"hello");\
        let mut decoder = StreamDecoder::new(&data);\
        assert_eq!(\
            decoder.read_string().unwrap(),\
            Primitive::String("hello".to_string())\
        );\
    }\
\
    #[test]\
    fn test_explicit_string_ref() {\
        let data = vec![0x55]; // ref 42 -> (42 << 1) | 1 = 85 (0x55)\
        let mut decoder = StreamDecoder::new(&data);\
        assert_eq!(decoder.read_string().unwrap(), Primitive::StringRef(42));\
    }\
\
    #[test]\
    fn test_explicit_timestamp() {\
        let ts_val: u64 = 1771622196842;\
        // encode ts_val as varint bytes\
        let mut data = Vec::new();\
        let mut val = ts_val;\
        loop {\
            let mut byte = (val & 0x7F) as u8;\
            val >>= 7;\
            if val != 0 {\
                byte |= 0x80;\
            }\
            data.push(byte);\
            if val == 0 {\
                break;\
            }\
        }\
        let mut decoder = StreamDecoder::new(&data);\
        let prim = decoder.read_timestamp().unwrap();\
        if let Primitive::Timestamp(dt) = prim {\
            assert_eq!(dt.timestamp_millis(), ts_val as i64);\
        } else {\
            panic!("Expected Timestamp primitive");\
        }\
    }\
' build-scan/lib/src/primitives.rs
