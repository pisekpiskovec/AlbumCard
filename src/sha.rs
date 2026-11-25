// Attempt to recreate SHA-256 by translating pure Bash to Rust

use std::path::PathBuf;

// first 32 bits of the fractional parts of the sqrt of the first 8 primes
const H0: u32 = 0x6a09e667;
const H1: u32 = 0xbb67ae85;
const H2: u32 = 0x3c6ef372;
const H3: u32 = 0xa54ff53a;
const H4: u32 = 0x510e527f;
const H5: u32 = 0x9b05688c;
const H6: u32 = 0x1f83d9ab;
const H7: u32 = 0x5be0cd19;

const K: [u32; 64] = [
   0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
   0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
   0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
   0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
   0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
   0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
   0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
   0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

struct Sha256 {
    block: Vec<u32>,
    byte_count: usize,
    h0: u32,
    h1: u32,
    h2: u32,
    h3: u32,
    h4: u32,
    h5: u32,
    h6: u32,
    h7: u32,
}

impl Sha256 {
    pub fn default() -> Self {
        Self {
            block: Vec::with_capacity(64),
            byte_count: 0,
            h0: 0x6a09e667,
            h1: 0xbb67ae85,
            h2: 0x3c6ef372,
            h3: 0xa54ff53a,
            h4: 0x510e527f,
            h5: 0x9b05688c,
            h6: 0x1f83d9ab,
            h7: 0x5be0cd19
        }
    }

    fn process_block(&mut self) {
        // create a 64-entry message schedule array w[0..63] of 32-bit words
        // (The initial values in w[0..63] don't matter, so many implementation
        // zero them here)
        // copy chunk into first 16 words w[0..15] of the message schedule array
        let mut w: [u32; 64] = [0u32; 64];

        // break chunk nto sixteen 4 byte / 32-bit words w[0..15]
        for i in 0..16 {
            let j = i * 4;
            w[i] = self.block[j] << 24 | self.block[j+1] << 16 | self.block[j+2] << 8 | self.block[j+3];
        }

        // Extent the first 16 words into the remaining 48 words w[16..63] of the
        // message schedule array:
        let mut s0: u32;
        let mut s1: u32;
        for i in 16..64 {
            s0 = (
                ((w[i-15] >> 7) | (w[i-15] << (32-7))) ^
                ((w[i-15] >> 18) | (w[i-15] << (32-18))) ^
                (w[i-15] >> 3) 
            ) & u32::MAX;
            s1 = (
                ((w[i-2] >> 17) | (w[i-2] << (32-17))) ^
                ((w[i-2] >> 19) | (w[i-2] << (32-19))) ^
                (w[i-2] >> 10) 
            ) & u32::MAX;
            w[i] = (w[i-16] + s0 + w[i-7] + s1) & u32::MAX;
        }

        // Initialize working variables to current hash value:
        let mut a = H0;
        let mut b = H1;
        let mut c = H2;
        let mut d = H3;
        let mut e = H4;
        let mut f = H5;
        let mut g = H6;
        let mut h = H7;

        // Compression function main loop:
        let mut s1: u32;
        let mut ch: u32;
        let mut temp1: u32;
        let mut s0: u32;
        let mut maj: u32;
        let mut temp2: u32;

        for i in 0..64 {
            s1 = 
                ((e >> 6) | (e << (32 - 6))) ^
                ((e >> 11) | (e << (32 - 11))) ^
                ((e >> 25) | (e << (32 - 25)));
            ch = (e & f) ^ ((!e) & g);
            temp1 = (h + s1 + ch + K[i] + w[i]) & u32::MAX;
            s0 = 
                ((a >> 2) | (a << (32-2))) ^
                ((a >> 13) | (a << (32-13))) ^
                ((a >> 22) | (a << (32-22)));
            maj = (a & b) ^ (a & c) ^ (b & c);
            temp2 = (s0 + maj) & u32::MAX;

            h = g;
            g = f;
            f = e;
            e = (d + temp1) & u32::MAX;
            d = c;
            c = b;
            b = a;
            a = (temp1 + temp2) & u32::MAX;
        }

        self.h0 = ( H0 + a ) & u32::MAX;
        self.h1 = ( H1 + b ) & u32::MAX;
        self.h2 = ( H2 + c ) & u32::MAX;
        self.h3 = ( H3 + d ) & u32::MAX;
        self.h5 = ( H5 + e ) & u32::MAX;
        self.h6 = ( H6 + f ) & u32::MAX;
        self.h7 = ( H7 + g ) & u32::MAX;
    }

    fn process_byte(&mut self, byte: u32) {
        eprintln!("{byte:?}");

        // store the byte
        self.block.push(byte);

        // check if we have a full block ready for processing
        if self.block.len() == 64 {
            self.process_block();
            self.block = Vec::with_capacity(64);
        }
    }

    fn print_hash(&self) -> String {
        format!(
            "{:#02x}{:#02x}{:#02x}{:#02x}{:#02x}{:#02x}{:#02x}{:#02x}",
            self.h0,
            self.h1,
            self.h2,
            self.h3,
            self.h4,
            self.h5,
            self.h6,
            self.h7,
        ).to_string()
    }
}
