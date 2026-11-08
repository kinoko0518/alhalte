// 上位4bit -> OR
// 下位4bit -> AND
#[derive(Clone, Copy)]
pub struct PermissionMask(u8);

impl PermissionMask {
    pub fn or_mask(self) -> u8 {
        self.0 >> 4
    }

    pub fn and_mask(self) -> u8 {
        0b11110000 | self.0
    }
}

#[derive(Clone, Copy)]
pub struct Permission(u8);

impl Permission {
    pub fn new(read: bool, write: bool, owner: bool) -> Self {
        Self((read as u8) << 2 | (write as u8) << 1 | (owner as u8))
    }

    pub fn apply_mask(self, mask: PermissionMask) -> Self {
        Self((self.0 | mask.or_mask()) & mask.and_mask())
    }
}
