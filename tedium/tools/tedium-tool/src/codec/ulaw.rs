
/*
fn ulaw_encode_cruddy(linear: f32) -> u8 {
    let linear = linear * 8192.0;
    let linear = (linear.clamp(-8192.0, 8191.0) as i16) as u16;
    let sign = ((linear >> 13) as u8) & 1;
    
    let mantissa = linear & ((1 << 13) - 1);
    assert!(mantissa < 8192);

    let leading_zeros = mantissa.leading_zeros() - 2;
    let code = if leading_zeros < 8 {
        let mantissa_shift = 8 - leading_zeros;
        let exponent = (7 - leading_zeros) as u8;
        (sign << 7) | (exponent << 4) | ((mantissa >> mantissa_shift) as u8 & 0x0f)
    } else {
        (sign << 7) | 0b000_0000
    };
    code ^ 0xff
}
*/
/*    
    match linear {
            4063 ..= i16::MAX => 0x80,
            2015 ..=     4062 => 0x90,
             991 ..=     2014 => 0xa0,
             479 ..=      990 => 0xb0,
             223 ..=      478 => 0xc0,
              95 ..=      222 => 0xd0,
              31 ..=       94 => 0xe0,
               1 ..=       30 => 0xf0,
               0              => 0xff,
              -1              => 0x7f,
             -31 ..=       -2 => 0x70,
             -95 ..=      -32 => 0x60,
            -223 ..=      -96 => 0x50,
            -479 ..=     -224 => 0x40,
            -991 ..=     -480 => 0x30,
           -2015 ..=     -992 => 0x20,
           -4063 ..=    -2016 => 0x10,
           -8159 ..=    -4064 => 0x00,
        i16::MIN ..=    -8160 => 0x00,
    }
*/

///////////////////////////////////////////////////////////////////////

/// G.191 reference implementation. Accurate but slow.
/// 
#[cfg(test)]
pub fn encode_ulaw_g191(linear: i16) -> u8 {
    // Using G.191 code as a reference, since G.711 is a little unclear in spots.

    /* -------------------------------------------------------------------- */
    /* Change from 14 bit left justified to 14 bit right justified */
    /* Compute absolute value; adjust for easy processing */
    /* -------------------------------------------------------------------- */
    let (absno, sign_bit) = if linear < 0 {
            /* compute 1's complement in case of */
            /* negative samples */
            (linear as u16 ^ 0xffff, 0x00)
        } else {
            (linear as u16, 0x80)
        };
    let absno = ((absno >> 2) + 33).clamp(0, 0x1fff);

    /* Determination of sample's segment */
    let mut i = absno >> 6;
    let mut segno = 1u8;
    while i != 0 {
        segno += 1;
        i >>= 1;
    }

    /* Mounting the high-nibble of the log-PCM sample */
    let high_nibble = 8 - segno;

    /* Mounting the low-nibble of the log PCM sample */
    let mut low_nibble = (absno >> segno) as u8       /* right shift of mantissa and */
                            & 0x000F;                /* masking away leading '1' */
    low_nibble = 0x000F - low_nibble;

    /* Joining the high-nibble and the low-nibble of the log PCM sample */
    sign_bit | (high_nibble << 4) | low_nibble
}

/// G.191 reference implementation. Accurate but slow.
/// 
#[cfg(test)]
pub fn decode_ulaw_g191(code: u8) -> i16 {
    let sign = if code < 0x80 { -1i16 } else { 1 };  /* sign-bit = 1 for positiv values */

    let mantissa = code ^ 0xff;      /* 1's complement of input value */
    let exponent = (mantissa >> 4) & 0x07;      /* extract exponent */
    let segment = exponent + 1;     /* compute segment number */
    let mantissa = mantissa & 0x0f;     /* extract mantissa */

    /* Compute Quantized Sample (14 bit left justified!) */
    let step = 4u16 << segment;      /* position of the LSB */
    /* = 1 quantization step) */
    sign *          /* sign */
        ((0x80 << exponent)   /* '1', preceding the mantissa */
        + step * mantissa as u16         /* left shift of mantissa */
        + step / 2               /* 1/2 quantization step */
        - 4 * 33) as i16
}

///////////////////////////////////////////////////////////////////////

static ULAW_TO_LINEAR: [i16; 256] = [
    // Table produced with G.191 code for G.711 u-law encoder.
    -32124, -31100, -30076, -29052, -28028, -27004, -25980, -24956,
    -23932, -22908, -21884, -20860, -19836, -18812, -17788, -16764,
    -15996, -15484, -14972, -14460, -13948, -13436, -12924, -12412,
    -11900, -11388, -10876, -10364,  -9852,  -9340,  -8828,  -8316,
     -7932,  -7676,  -7420,  -7164,  -6908,  -6652,  -6396,  -6140,
     -5884,  -5628,  -5372,  -5116,  -4860,  -4604,  -4348,  -4092,
     -3900,  -3772,  -3644,  -3516,  -3388,  -3260,  -3132,  -3004,
     -2876,  -2748,  -2620,  -2492,  -2364,  -2236,  -2108,  -1980,
     -1884,  -1820,  -1756,  -1692,  -1628,  -1564,  -1500,  -1436,
     -1372,  -1308,  -1244,  -1180,  -1116,  -1052,   -988,   -924,
      -876,   -844,   -812,   -780,   -748,   -716,   -684,   -652,
      -620,   -588,   -556,   -524,   -492,   -460,   -428,   -396,
      -372,   -356,   -340,   -324,   -308,   -292,   -276,   -260,
      -244,   -228,   -212,   -196,   -180,   -164,   -148,   -132,
      -120,   -112,   -104,    -96,    -88,    -80,    -72,    -64,
       -56,    -48,    -40,    -32,    -24,    -16,     -8,      0,     // Interesting this zero is a "-1" according to other "authorities".
     32124,  31100,  30076,  29052,  28028,  27004,  25980,  24956,
     23932,  22908,  21884,  20860,  19836,  18812,  17788,  16764,
     15996,  15484,  14972,  14460,  13948,  13436,  12924,  12412,
     11900,  11388,  10876,  10364,   9852,   9340,   8828,   8316,
      7932,   7676,   7420,   7164,   6908,   6652,   6396,   6140,
      5884,   5628,   5372,   5116,   4860,   4604,   4348,   4092,
      3900,   3772,   3644,   3516,   3388,   3260,   3132,   3004,
      2876,   2748,   2620,   2492,   2364,   2236,   2108,   1980,
      1884,   1820,   1756,   1692,   1628,   1564,   1500,   1436,
      1372,   1308,   1244,   1180,   1116,   1052,    988,    924,
       876,    844,    812,    780,    748,    716,    684,    652,
       620,    588,    556,    524,    492,    460,    428,    396,
       372,    356,    340,    324,    308,    292,    276,    260,
       244,    228,    212,    196,    180,    164,    148,    132,
       120,    112,    104,     96,     88,     80,     72,     64,
        56,     48,     40,     32,     24,     16,      8,      0,
];

pub fn decode_i16(code: u8) -> i16 {
    ULAW_TO_LINEAR[code as usize]
}

pub fn encode_i14(linear: i16) -> u8 {
    let linear = linear.clamp(-8031, 8031) as u16;
    let sign_extended = (0 - ((linear >> 15) as i16)) as u16;  // 0x0000 if positive, 0xffff if negative.
    let magnitude = ((linear << 3) ^ sign_extended) + (33 << 3);

    let leading_zeros = magnitude.leading_zeros();
    debug_assert!(leading_zeros < 8);

    let abcd = (magnitude >> (11 - leading_zeros)) as u8 & 0b1111;
    let exponent = 7 - leading_zeros;
    debug_assert!(exponent < 8);

    let code = (sign_extended as u8 & 0x80) | ((exponent as u8) << 4) | abcd;
    code ^ 0xff
}


const SCALE_ULAW_FLOAT: f32 = 8159.0;

/// Map +/-1.0 to uLaw full-scale (+/-8159 or +3.17dBm0)
/// 
pub fn encode(linear: f32) -> u8 {
    let clamped = linear.clamp(-1.0, 1.0);
    let scaled = clamped * SCALE_ULAW_FLOAT;
    let sample_i14 = scaled.round() as i16;
    encode_i14(sample_i14)
}

pub fn decode(code: u8) -> f32 {
    let linear = decode_i16(code);
    linear as f32 / (SCALE_ULAW_FLOAT * 4.0)
}

///////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    #[test]
    fn decoder_vs_g711_table() {
        let expected = [
            // G.711 table 2a
            (0b1000_0000,  8031),
            (0b1000_1111,  4191),
            (0b1001_1111,  2079),
            (0b1010_1111,  1023),
            (0b1011_1111,   495),
            (0b1100_1111,   231),
            (0b1101_1111,    99),
            (0b1110_1111,    33),
            (0b1111_1110,     2),
            (0b1111_1111,     0),

            // G.711 table 2b
            (0b0111_1111,     0),
            (0b0111_1110,    -2),
            (0b0110_1111,   -33),
            (0b0101_1111,   -99),
            (0b0100_1111,  -231),
            (0b0011_1111,  -495),
            (0b0010_1111, -1023),
            (0b0001_1111, -2079),
            (0b0000_1111, -4191),
            (0b0000_0001, -7775),
            (0b0000_0000, -8031),
        ];

        for (character_signal, linear) in expected {
            let decoder_output = super::decode_ulaw_g191(character_signal);
            let decoder_output_14_bit = decoder_output / 4;
            assert_eq!(linear, decoder_output_14_bit, "decoding pair {:?}", (character_signal, linear));
        }
    }

    #[test]
    fn encoder_vs_g191_code() {
        let expected = [
            // G.711 table 2a, viewed through the clarifying lens of G.726 and G.191
            ( 8191, 0b1000_0000),
            ( 8159, 0b1000_0000),
            ( 8031, 0b1000_0000),
            ( 7903, 0b1000_0000),
            ( 7902, 0b1000_0001),
            ( 4191, 0b1000_1111),
            ( 4063, 0b1000_1111),
            ( 4062, 0b1001_0000),
            ( 2143, 0b1001_1110),
            ( 2079, 0b1001_1111),
            ( 2015, 0b1001_1111),
            ( 1055, 0b1010_1110),
            ( 1023, 0b1010_1111),
            (  991, 0b1010_1111),
            (  511, 0b1011_1110),
            (  495, 0b1011_1111),
            (  479, 0b1011_1111),
            (  239, 0b1100_1110),
            (  231, 0b1100_1111),
            (  223, 0b1100_1111),
            (  103, 0b1101_1110),
            (   99, 0b1101_1111),
            (   95, 0b1101_1111),
            (   35, 0b1110_1110),
            (   33, 0b1110_1111),
            (   31, 0b1110_1111),
            (    3, 0b1111_1101),
            (    2, 0b1111_1110),
            (    1, 0b1111_1110),
            (    0, 0b1111_1111),

            // G.711 table 2b, viewed through the clarifying lens of G.726 and G.191
            (   -1, 0b0111_1111),
            (   -2, 0b0111_1110),
            (   -3, 0b0111_1110),
            (  -31, 0b0111_0000),
            (  -33, 0b0110_1111),
            (  -35, 0b0110_1111),
            (  -95, 0b0110_0000),
            (  -99, 0b0101_1111),
            ( -103, 0b0101_1111),
            ( -223, 0b0101_0000),
            ( -231, 0b0100_1111),
            ( -239, 0b0100_1111),
            ( -479, 0b0100_0000),
            ( -495, 0b0011_1111),
            ( -511, 0b0011_1111),
            ( -991, 0b0011_0000),
            (-1023, 0b0010_1111),
            (-1055, 0b0010_1111),
            (-2015, 0b0010_0000),
            (-2079, 0b0001_1111),
            (-2143, 0b0001_1111),
            (-4063, 0b0001_0000),
            (-4191, 0b0000_1111),
            (-4319, 0b0000_1111),
            (-7647, 0b0000_0010),
            (-7775, 0b0000_0001),
            (-7903, 0b0000_0001),
            (-8031, 0b0000_0000),
            (-8059, 0b0000_0000),
            (-8191, 0b0000_0000),
        ];

        for (linear, character_signal) in expected {
            // let linear_normalized = linear * 4;
            let encoder_output = super::encode_i14(linear);
            assert_eq!(encoder_output, character_signal, "encoding pair {:?}", (linear, character_signal));
        }
    }
}
