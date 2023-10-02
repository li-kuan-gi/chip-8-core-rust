pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_V_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

const START_ADDR: u16 = 0x200;

const FONTSET_SIZE: usize = 80;

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Emu {
    pc: u16, // program counter
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_reg: [u8; NUM_V_REGS],
    i_reg: u16, // index register
    stack: [u16; STACK_SIZE],
    sp: u16, // stack pointer
    dt: u8,  // delay timer
    st: u8,  // sound timer
    keys: [bool; NUM_KEYS],
}

impl Emu {
    pub fn new() -> Self {
        let mut emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0; NUM_V_REGS],
            i_reg: 0,
            stack: [0; STACK_SIZE],
            sp: 0,
            dt: 0,
            st: 0,
            keys: [false; NUM_KEYS],
        };

        emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        emu
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_V_REGS];
        self.i_reg = 0;
        self.stack = [0; STACK_SIZE];
        self.sp = 0;
        self.dt = 0;
        self.st = 0;
        self.keys = [false; NUM_KEYS];

        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self) {
        // Fetch
        let op = self.fetch();

        // Decode & execute
        self.execute(op);
    }

    // Called once per frame
    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            if self.st == 1 {
                // beep
            }
            self.st -= 1;
        }
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, index: usize, pressed: bool) {
        self.keys[index] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = start + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn push(&mut self, value: u16) {
        self.stack[self.sp as usize] = value;
        self.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;

        self.pc += 2;

        // Big Endian
        (higher_byte << 8) | lower_byte
    }

    fn execute(&mut self, op: u16) {
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            // NOP
            (0, 0, 0, 0) => {}

            // CLS, clear screen
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            }

            // RET, return from subroutine
            (0, 0, 0xE, 0xE) => {
                let return_addr = self.pop();
                self.pc = return_addr;
            }

            // JMP NNN, jump
            (1, _, _, _) => {
                let nnn = op & 0xFFF;
                self.pc = nnn;
            }

            // CALL NNN, call subroutine (and then jump)
            (2, _, _, _) => {
                let nnn = op & 0xFFF;
                self.push(self.pc);
                self.pc = nnn;
            }

            // Skip next opcode if VX == NN
            (3, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;

                if self.v_reg[x] == nn {
                    self.pc += 2;
                }
            }

            // Skip next opcode if VX != NN
            (4, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;

                if self.v_reg[x] != nn {
                    self.pc += 2;
                }
            }

            // Skip next opcode if VX == VY
            (5, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                if self.v_reg[x] == self.v_reg[y] {
                    self.pc += 2;
                }
            }

            // VX = NN
            (6, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                self.v_reg[x] = nn;
            }

            // VX += NN, doesn't affect carry flag
            (7, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                self.v_reg[x] = self.v_reg[x].wrapping_add(nn);
            }

            // VX = VY
            (8, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] = self.v_reg[y];
            }

            // VX |= VY
            (8, _, _, 1) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] |= self.v_reg[y];
            }

            // VX &= VY
            (8, _, _, 2) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] &= self.v_reg[y];
            }

            // VX ^= VY
            (8, _, _, 3) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] ^= self.v_reg[y];
            }

            // VX += VY; set VF if carry
            (8, _, _, 4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (result, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);

                self.v_reg[x] = result;
                self.v_reg[0xF] = if carry { 1 } else { 0 };
            }

            // VX -= VY; clear VF if borrow
            (8, _, _, 5) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (result, borrow) = self.v_reg[x].overflowing_sub(self.v_reg[y]);

                self.v_reg[x] = result;
                self.v_reg[0xF] = if borrow { 0 } else { 1 };
            }

            // VX >>= 1; store dropped bit in VF
            (8, _, _, 6) => {
                let x = digit2 as usize;
                let dropped = self.v_reg[x] * 1;
                self.v_reg[x] >>= 1;
                self.v_reg[0xF] = dropped;
            }

            // VX = VY - VX; clear VF if borrow
            (8, _, _, 7) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (result, borrow) = self.v_reg[y].overflowing_sub(self.v_reg[x]);

                self.v_reg[x] = result;
                self.v_reg[0xF] = if borrow { 0 } else { 1 };
            }

            // VX <<= 1; store dropped bit in VF
            (8, _, _, 0xE) => {
                let x = digit2 as usize;
                let dropped = (self.v_reg[x] >> 7) & 1;
                self.v_reg[x] <<= 1;
                self.v_reg[0xF] = dropped;
            }

            // Skip next opcode if VX != VY
            (9, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                if self.v_reg[x] != self.v_reg[y] {
                    self.pc += 2;
                }
            }

            // I = NNN
            (0xA, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.i_reg = nnn;
            }

            // Jump to V0 + NNN
            (0xB, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.pc = (self.v_reg[0] as u16) + nnn;
            }

            // VX = rand() & NN
            (0xC, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                let rnd: u8 = rand::random();
                self.v_reg[x] = rnd & nn;
            }

            // Draw sprite at (VX, VY), N pixels tall, XORed onto screen, VF set if any erased
            (0xD, _, _, _) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let n = digit4 as usize;
                let mut flipped = false;

                for delta_y in 0..n {
                    let flips = self.ram[(self.i_reg as usize) + delta_y];

                    for delta_x in 0..8 {
                        let flip = flips & (0x80 >> delta_x) != 0;

                        if flip {
                            let x = (x + delta_x) % SCREEN_WIDTH;
                            let y = (y + delta_y) % SCREEN_HEIGHT;

                            let index = y * SCREEN_WIDTH + x;

                            flipped |= self.screen[index];
                            self.screen[index] ^= true;
                        }
                    }
                }

                self.v_reg[0xF] = if flipped { 1 } else { 0 };
            }

            // Skip next opcode if key index in VX is pressed
            (0xE, _, 0x9, 0xE) => {
                let x = digit2 as usize;
                let key_index = self.v_reg[x] as usize;
                let pressed = self.keys[key_index];

                if pressed {
                    self.pc += 2;
                }
            }

            // Skip next opcode if key index in VX is not pressed
            (0xE, _, 0xA, 0x1) => {
                let x = digit2 as usize;
                let key_index = self.v_reg[x] as usize;
                let pressed = self.keys[key_index];

                if !pressed {
                    self.pc += 2;
                }
            }

            // VX = Delay Timer
            (0xF, _, 0x0, 0x7) => {
                let x = digit2 as usize;
                self.v_reg[x] = self.dt;
            }

            // Waits for key press, store index in VX, blocking
            (0xF, _, 0x0, 0xA) => {
                let x = digit2 as usize;
                let mut pressed = false;

                for index in 0..NUM_KEYS {
                    if self.keys[index] {
                        self.v_reg[x] = index as u8;
                        pressed = true;
                        break;
                    }
                }

                if !pressed {
                    // Redo opcode
                    self.pc -= 2;
                }
            }

            // Delay Timer = VX
            (0xF, _, 0x1, 0x5) => {
                let x = digit2 as usize;
                self.dt = self.v_reg[x];
            }

            // Sound Timer
            (0xF, _, 0x1, 0x8) => {
                let x = digit2 as usize;
                self.st = self.v_reg[x];
            }

            // I += VX
            (0xF, _, 0x1, 0xE) => {
                let x = digit2 as usize;
                self.i_reg = self.i_reg.wrapping_add(self.v_reg[x] as u16);
            }

            // I = address of font character in VX
            (0xF, _, 0x2, 0x9) => {
                let x = digit2 as usize;
                let c = self.v_reg[x];
                self.i_reg = 5 * c as u16;
            }

            // Store BCD encoding of VX inot I
            (0xF, _, 0x3, 0x3) => {
                let x = digit2 as usize;
                let num = self.v_reg[x];

                for i in 0..3 {
                    let digit = (num / u8::pow(10, i)) % 10;
                    let addr = (self.i_reg + i as u16) as usize;
                    self.ram[addr] = digit;
                }
            }

            // Store V0 thru VX into RAM address starting at I (inclusive)
            (0xF, _, 0x5, 0x5) => {
                let x = digit2 as usize;

                for i in 0..=x {
                    let addr = (self.i_reg as usize) + i;
                    self.ram[addr] = self.v_reg[i];
                }
            }

            // Fill V0 thru VX with RAM values starting at I (inclusive)
            (0xF, _, 0x6, 0x5) => {
                let x = digit2 as usize;
                for i in 0..=x {
                    let addr = (self.i_reg as usize) + i;
                    self.v_reg[i] = self.ram[addr];
                }
            }

            // unimplemented opcode
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", op),
        }
    }
}
