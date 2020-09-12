use core::num::NonZeroUsize;
use core::ptr;

use ufmt::{uwrite, uWrite};

use primeapi::mp1::{CGameState, CPlayerState, CRelayTracker, CWorldState};

struct FmtCursor<'a>
{
    idx: usize,
    buf: &'a mut [u8]
}

impl<'a> FmtCursor<'a>
{
    pub fn new(buf: &'a mut [u8]) -> Self
    {
        FmtCursor {
            idx: 0,
            buf,
        }
    }

    pub fn position(&self) -> usize
    {
        self.idx
    }

    pub fn set_position(&mut self, pos: usize)
    {
        self.idx = pos
    }
}

struct FmtCursorBufOverrunError;

impl<'a> uWrite for FmtCursor<'a>
{
    type Error = FmtCursorBufOverrunError;
    fn write_str(&mut self, s: &str) -> Result<(), Self::Error>
    {
        if s.len() + self.idx > self.buf.len() {
            Err(FmtCursorBufOverrunError)
        } else {
            self.buf[self.idx..self.idx + s.len()].copy_from_slice(s.as_bytes());
            self.idx += s.len();
            Ok(())
        }
    }
}


const PICKUP_MEMORY_RELAY_IDS: &[(u32, &[(u8, u32)])] = &[
    (0x83f6ff6f, &[
        (0, 131373),
        (1, 131378),
        (2, 131179),
        (3, 131417),
        (4, 524407),
        (5, 589864),
        (6, 589929),
        (7, 589934),
        (8, 720958),
        (9, 786477),
        (10, 1048580),
        (11, 1179652),
        (12, 1310958),
        (13, 1377078),
        (14, 1769498),
        (15, 1835055),
        (16, 1835105),
        (17, 1966451),
        (18, 2097240),
        (19, 2359773),
        (20, 2435311),
        (21, 405090452),
        (21, 405090174),
        (22, 2490377),
        (23, 2687110),
        (24, 2697191),
        (25, 2949155),
        (26, 2949294),
        (27, 3145783),
        (28, 3145755),
        (28, 3155851),
        (29, 3211363),
        (30, 3211276),
        (31, 3408607),
        (32, 3474121),
        (33, 3735556),
        (34, 3997700),
    ]),
    (0xa8be6291, &[
        (35, 131439),
        (36, 131447),
        (37, 524888),
        (38, 600302),
        (39, 655532),
        (40, 655762),
        (41, 917667),
        (41, 917593),
        (42, 983597),
        (43, 1049222),
        (43, 1048803),
        (44, 1573324),
        (45, 1639700),
        (46, 1769490),
        (47, 1966839),
        (48, 2031782),
        (49, 2557136),
        (50, 69730589),
        (51, 2687368),
        (52, 3343329),
        (53, 3343378),
        (54, 3473441),
        (55, 3473709),
        (56, 3539114),
        (57, 3604506),
    ]),
    (0x39f2de28, &[
        (58, 133),
        (59, 262158),
        (60, 524796),
        (61, 852167),
        (62, 983294),
        (63, 68157909),
        (64, 1245495),
        (65, 1310742),
        (66, 1769750),
        (67, 1966829),
        (68, 2293844),
        (69, 2424846),
        (70, 2555959),
        (71, 2752546),
        (72, 2753077),
    ]),
    (0xb1ac4d65, &[
        (73, 131636),
        (74, 328072),
        (75, 589962),
        (75, 589829),
        (76, 786471),
        (77, 852801),
        (78, 853234),
        (79, 983182),
        (80, 1179918),
        (81, 1247079),
        (82, 1441960),
        (83, 1705145),
        (84, 1770865),
        (84, 1770863),
        (84, 1770674),
        (85, 2032134),
        (86, 2098667),
        (87, 2359592),
        (88, 2556032),
        (89, 2621699),
    ]),
    (0x3ef8237c, &[
        (90, 272509),
        (91, 393485),
        (92, 524304),
        (93, 655428),
        (94, 720952),
        (95, 786473),
        (96, 917979),
        (97, 918080),
        (98, 1376288),
        (99, 1507983),
    ])
];

#[derive(Clone, Debug, PartialEq)]
struct ItemsCollected([u8; 13]);
impl ItemsCollected
{
    fn new() -> Self
    {
        ItemsCollected([0; 13])
    }

    fn set(&mut self, i: u8, val: bool)
    {
        if i >= 100 {
            return;
        }
        if val {
            self.0[(i >> 3) as usize] |= 1 << (i & 3);
        } else {
            self.0[(i >> 3) as usize] &= !(1 << (i & 3));
        }
    }

    fn test(&self, i: u8) -> bool
    {
        (self.0[(i >> 3) as usize] & (1 << (i & 3))) != 0
    }
}

fn current_items_collected(game_state: *mut CGameState) -> ItemsCollected
{
    let mut locs = ItemsCollected::new();
    let world_states = unsafe { &*CGameState::world_states(game_state) };
    for world_state in world_states.iter() {
        let mlvl_relays = PICKUP_MEMORY_RELAY_IDS.iter()
            .find(|(mlvl, _)| *mlvl == unsafe { CWorldState::mlvl(world_state) });
        let mlvl_relays = if let Some(mlvl_relays) = mlvl_relays {
            mlvl_relays.1
        } else {
            continue
        };

        let relay_tracker = unsafe { CWorldState::relay_tracker(world_state) };
        if !relay_tracker.is_null() && !unsafe { *relay_tracker }.is_null() {
            let relay_tracker = unsafe { *relay_tracker };
            let relays = unsafe { &*CRelayTracker::relays(relay_tracker) };

            for relay in relays.iter() {
                if let Some((loc_idx, _)) = mlvl_relays.iter().find(|(_, id)| id == relay) {
                    locs.set(*loc_idx, true);
                }
            }
        }
    }

    locs
}

#[derive(Clone)]
pub struct TrackerState
{
    play_time: f64,
    play_time_paused: bool,
    items_collected: ItemsCollected,
    inventory: [u8; 41],
}

impl TrackerState
{
    pub fn new() -> Self {
        TrackerState {
            play_time: 0.0,
            play_time_paused: false,
            items_collected: ItemsCollected::new(),
            inventory: [0; 41],
        }
    }
}

pub fn update_tracker_state(ts: &mut TrackerState, initial: bool, time_only: bool, msg_buf: &mut [u8])
    -> Option<NonZeroUsize>
{
    let game_state = CGameState::global_instance();
    if game_state.is_null() {
        return None;
    }

    let player_state_ptr = unsafe { CGameState::player_state(game_state) };
    if player_state_ptr.is_null() {
        return None;
    }
    let player_state = unsafe { ptr::read(player_state_ptr) };
    if player_state.is_null() {
        return None;
    }

    let curr_play_time = unsafe { CGameState::play_time(game_state) };
    let play_time_updated = if ts.play_time_paused {
        if curr_play_time != ts.play_time {
            ts.play_time_paused = false;
            true
        } else {
            false
        }
    } else {
        if curr_play_time == ts.play_time {
            ts.play_time_paused = true;
            true
        } else {
            false
        }
    };
    ts.play_time = curr_play_time;

    let items_collected_updated;
    let inventory_updated;
    if time_only {
        items_collected_updated = false;
        inventory_updated = false;
    } else {
        let items_collected = current_items_collected(game_state);
        items_collected_updated = ts.items_collected != items_collected;
        ts.items_collected = items_collected;

        let mut inventory = [0u8; 41];
        for (i, inventory_slot) in inventory.iter_mut().enumerate() {
            let cap = unsafe { CPlayerState::get_item_capacity(player_state, i as i32) };
            *inventory_slot = cap as u8;
        }
        inventory_updated = ts.inventory[..] != inventory[..];
        ts.inventory = inventory;
    }

    if !initial && !play_time_updated && !items_collected_updated && !inventory_updated {
        return None;
    }

    let mut f = FmtCursor::new(msg_buf);
    let r: Result<_, FmtCursorBufOverrunError> = (|| {
        uwrite!(f, "{{")?;
        if play_time_updated || initial {
            uwrite!(f, "\"play_time\":{{")?;
            if ts.play_time_paused {
                uwrite!(f, "\"paused\":")?;
            } else {
                uwrite!(f, "\"running\":")?;
            }
            uwrite!(f, "{}", (ts.play_time * 100.0) as u32)?;
            uwrite!(f, "}},")?;
        }
        if items_collected_updated || initial {
            uwrite!(f, "\"items_collected\":[")?;
            for i in 0..100 {
                if ts.items_collected.test(i) {
                    uwrite!(f, "1,")?;
                } else {
                    uwrite!(f, "0,")?;
                }
            }
            // Clear the trailing comma
            f.set_position(f.position() - 1);
            uwrite!(f, "],")?;
        }
        if inventory_updated || initial {
            uwrite!(f, "\"inventory\":[")?;
            for i in ts.inventory.iter() {
                uwrite!(f, "{},", i)?;
            }
            // Clear the trailing comma
            f.set_position(f.position() - 1);
            uwrite!(f, "],")?;
        }
        // Clear the trailing comma
        f.set_position(f.position() - 1);
        uwrite!(f, "}}")?;
        Ok(f.position())
    })();
    r.ok().and_then(|i| NonZeroUsize::new(i))
}
