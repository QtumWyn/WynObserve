use iced_x86::{Decoder, DecoderOptions, Formatter, NasmFormatter};

#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    pub ip: u64,
    pub bytes: Vec<u8>,
    pub text: String,
}

pub fn decode_one(ip: u64, bytes: &[u8]) -> Result<DecodedInstruction, String> {
    if bytes.is_empty() {
        return Err("no instruction bytes available".to_string());
    }

    let mut decoder = Decoder::with_ip(64, bytes, ip, DecoderOptions::NONE);

    let instruction = decoder.decode();

    if instruction.is_invalid() {
        return Err(format!("invalid instruction at 0x{ip:016X}"));
    }

    let instruction_len = instruction.len();

    if instruction_len > bytes.len() {
        return Err(format!(
            "instruction at 0x{ip:016X} \
             requires {instruction_len} bytes, \
             only {} available",
            bytes.len(),
        ));
    }

    let mut formatter = NasmFormatter::new();

    let mut text = String::new();

    formatter.format(&instruction, &mut text);

    Ok(DecodedInstruction {
        ip,
        bytes: bytes[..instruction_len].to_vec(),
        text,
    })
}
