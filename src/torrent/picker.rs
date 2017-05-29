use std::collections::{HashSet, HashMap};
use torrent::{PieceField, Info, Peer};

pub struct Picker {
    endgame_cnt: u32,
    piece_idx: u32,
    pieces: PieceField,
    scale: u32,
    waiting: HashSet<u32>,
    waiting_peers: HashMap<u32, HashSet<usize>>,
}

impl Picker {
    pub fn new(info: &Info) -> Picker {
        let scale = info.piece_len/16384;
        // The n - 1 piece length, since the last one is (usually) shorter.
        let compl_piece_len = scale * (info.pieces() as usize - 1);
        // the nth piece length
        let mut last_piece_len = info.total_len - info.piece_len * (info.pieces() as usize - 1);
        if last_piece_len % 16384 == 0 {
            last_piece_len /= 16384;
        } else {
            last_piece_len /= 16384;
            last_piece_len += 1;
        }
        let len = compl_piece_len + last_piece_len;
        let pieces = PieceField::new(len as u32);
        Picker {
            pieces,
            piece_idx: 0,
            scale: scale as u32,
            waiting: HashSet::new(),
            endgame_cnt: len as u32,
            waiting_peers: HashMap::new(),
        }
    }

    pub fn pick(&mut self, peer: &Peer) -> Option<(u32, u32)> {
        for idx in peer.pieces.iter_from(self.piece_idx) {
            let start = idx * self.scale;
            for i in 0..self.scale {
                // On the last piece check, we won't check the whole range.
                if start + i < self.pieces.len() && !self.pieces.has_piece(start + i) {
                    self.pieces.set_piece(start + i);
                    self.waiting.insert(start + i);
                    let mut hs = HashSet::with_capacity(1);
                    hs.insert(peer.id);
                    self.waiting_peers.insert(start + i, hs);
                    if self.endgame_cnt == 1 {
                        println!("Entering endgame!");
                    }
                    self.endgame_cnt = self.endgame_cnt.saturating_sub(1);
                    return Some((idx, i * 16384));
                }
            }
        }
        if self.endgame_cnt == 0 {
            let mut idx = None;
            for piece in self.waiting.iter() {
                if peer.pieces.has_piece(*piece/self.scale) {
                    idx = Some(*piece);
                    break;
                }
            }
            if let Some(i) = idx {
                self.waiting_peers.get_mut(&i).unwrap().insert(peer.id);
                return Some((i/self.scale, (i % self.scale) * 16384));
            }
        }
        None
    }

    /// Returns whether or not the whole piece is complete.
    pub fn completed(&mut self, mut idx: u32, mut offset: u32) -> (bool, HashSet<usize>) {
        offset /= 16384;
        idx *= self.scale;
        self.waiting.remove(&(idx + offset));
        // TODO: make this less hacky
        let peers = self.waiting_peers.remove(&(idx + offset)).unwrap_or(HashSet::with_capacity(0));
        for i in 0..self.scale {
            if (idx + i < self.pieces.len() && !self.pieces.has_piece(idx + i)) || self.waiting.contains(&(idx + i)) {
                return (false, peers);
            }
        }
        self.update_piece_idx();
        (true, peers)
    }

    fn update_piece_idx(&mut self) {
        let mut idx = self.piece_idx * self.scale;
        loop {
            for i in 0..self.scale {
                if (idx + i < self.pieces.len() && !self.pieces.has_piece(idx + i)) || self.waiting.contains(&(idx + i)) {
                    return;
                }
            }
            self.piece_idx += 1;
            idx += self.scale;
            if idx > self.pieces.len() {
                return;
            }
        }
    }
}

#[test]
fn test_piece_size() {
    let info = Info {
        announce: String::from(""),
        piece_len: 262144,
        total_len: 2000000,
        hashes: vec![vec![0u8]; 8],
        hash: [0u8; 20],
        files: vec![],
    };

    let mut picker = Picker::new(&info);
    assert_eq!(picker.scale as usize, info.piece_len/16384);
    assert_eq!(picker.pieces.len(), 123);
}
