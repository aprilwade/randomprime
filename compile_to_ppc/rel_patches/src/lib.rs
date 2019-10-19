#![feature(try_blocks)]
#![feature(type_alias_impl_trait)]
#![no_std]

extern crate alloc;

use futures::future;
use futures::never::Never;
use linkme::distributed_slice;
use pin_utils::pin_mut;

use primeapi::{patch_fn, prolog_fn};
use primeapi::mp1::{
    CArchitectureQueue, CGameState, CGuiFrame, CGuiTextSupport, CGuiTextPane, CGuiWidget,
    CMainFlow, CPlayerState, CRelayTracker, CStringTable, CWorldState,
};
use primeapi::rstl::WString;
use primeapi::alignment_utils::Aligned32;

use alloc::boxed::Box;

use core::fmt::Write;
use core::future::Future;
use core::mem::MaybeUninit;
use core::num::NonZeroUsize;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

mod ipc_async;
mod sock_async;
mod http;


pub fn delay(ticks: u32) -> impl Future<Output = ()>
{
    extern "C" {
        fn OSGetTime() -> u64;
    }

    let finished = ticks as u64 + unsafe { OSGetTime() };
    async_utils::poll_until(move || unsafe { OSGetTime() } >= finished)
}

pub fn milliseconds_to_ticks(ms: u32) -> u32
{
    const TB_BUS_CLOCK: u32 = 162000000;
    // const TB_CORE_CLOCK: u32 = 486000000;
    const TB_TIMER_CLOCK: u32 = (TB_BUS_CLOCK / 4000);


    ms * TB_TIMER_CLOCK
}

static mut EVENT_LOOP: Option<Pin<Box<dyn Future<Output = Never>>>> = None;

#[prolog_fn]
unsafe extern "C" fn setup_global_state()
{
    debug_assert!(EVENT_LOOP.is_none());
    EVENT_LOOP = Some(Box::pin(event_loop()));
    primeapi::dbg!(core::mem::size_of_val(&event_loop()));
}


#[patch_fn(kind = "call",
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x2c)]
unsafe extern "C" fn update_main_menu_text(frame: *mut CGuiFrame, widget_name: *const u8)
    -> *mut CGuiWidget
{
    let res = CGuiFrame::find_widget(frame, widget_name);

    let raw_string = CStringTable::get_string(CStringTable::main_string_table(), 110);
    let s = WString::from_ucs2_str(raw_string);

    for name in &[b"textpane_identifier\0".as_ptr(), b"textpane_identifierb\0".as_ptr()] {
        let widget = CGuiFrame::find_widget(frame, *name);
        let text_support = CGuiTextPane::text_support_mut(widget as *mut CGuiTextPane);
        CGuiTextSupport::set_text(text_support, &s);
    }

    res
}

#[patch_fn(kind = "return",
           target = "Draw__13CIOWinManagerCFv" + 0x124)]
unsafe extern "C" fn hook_every_frame()
{
    if ipc_async::running_on_dolphin() {
        return
    }

    let event_loop = if let Some(event_loop) = EVENT_LOOP.as_mut() {
        event_loop
    } else {
        return
    };
    let waker = async_utils::empty_waker();
    let mut ctx = Context::from_waker(&waker);
    match event_loop.as_mut().poll(&mut ctx) {
        Poll::Pending => (),
        Poll::Ready(never) => match never { },
    }
}

pub struct FmtCursor<'a>
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

impl<'a> core::fmt::Write for FmtCursor<'a>
{
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error>
    {
        if s.len() + self.idx > self.buf.len() {
            Err(core::fmt::Error)
        } else {
            self.buf[self.idx..self.idx + s.len()].copy_from_slice(s.as_bytes());
            self.idx += s.len();
            Ok(())
        }
    }
}

async fn event_loop() -> Never
{
    let ss = sock_async::SockSystem::new().await;
    let addr = sock_async::SockAddr {
        len: 8,
        family: sock_async::AF_INET as u8,
        port: 80,
        name: sock_async::INADDR_ANY,
        unused: Default::default(),
    };
    let mut queue = async_utils::FutureQueue::<_, generic_array::typenum::U5>::new();
    let (queue_poller, mut queue_pusher) = queue.split();
    let mut server = ss.tcp_listen(&addr, 1).await.unwrap();
    let connect_fut = async {
        loop {
            // TODO: Allow multiple clients, maybe with a vec?
            //       Set a recv timeout on the client for 30s or 60s
            let mut client = server.accept().await.unwrap();

            let fut = Box::pin(async move {
                let (send, recv) = client.split();
                let mut buf = Aligned32::new([MaybeUninit::uninit(); 4096]);
                match crate::http::handle_http_request(&mut buf[..], recv, send).await {
                    Ok(_) => {
                        primeapi::dbg!("successful http request");
                    },
                    Err(crate::http::HttpRequestError::ReaderIO(e)) => {
                        primeapi::dbg!("Reader IO Error", e);
                    },
                    Err(crate::http::HttpRequestError::WriterIO(e)) => {
                        primeapi::dbg!("Writer IO Error", e);
                    },
                    Err(crate::http::HttpRequestError::Internal) => {
                        primeapi::dbg!("failed http request");
                    },
                }
            });
            queue_pusher.push(fut).await;
        }
    };
    pin_mut!(connect_fut);
    future::select(
        connect_fut,
        queue_poller
    ).await.factor_first().0
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
struct TrackerState
{
    play_time: f64,
    play_time_paused: bool,
    items_collected: ItemsCollected,
    inventory: [u8; 41],
}

impl TrackerState
{
    fn new() -> Self {
        TrackerState {
            play_time: 0.0,
            play_time_paused: false,
            items_collected: ItemsCollected::new(),
            inventory: [0; 41],
        }
    }
}

fn update_tracker_state(ts: &mut TrackerState, initial: bool, time_only: bool, msg_buf: &mut [u8])
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
    let r: Result<_, core::fmt::Error> = (|| {
        write!(f, "{{")?;
        if play_time_updated || initial {
            write!(f, "\"play_time\":{{")?;
            if ts.play_time_paused {
                write!(f, "\"paused\":")?;
            } else {
                write!(f, "\"running\":")?;
            }
            write!(f, "{}", (ts.play_time * 100.0) as u32)?;
            write!(f, "}},")?;
        }
        if items_collected_updated || initial {
            write!(f, "\"items_collected\":[")?;
            for i in 0..100 {
                if ts.items_collected.test(i) {
                    write!(f, "1,")?;
                } else {
                    write!(f, "0,")?;
                }
            }
            // Clear the trailing comma
            f.set_position(f.position() - 1);
            write!(f, "],")?;
        }
        if inventory_updated || initial {
            write!(f, "\"inventory\":[")?;
            for i in ts.inventory.iter() {
                write!(f, "{},", i)?;
            }
            // Clear the trailing comma
            f.set_position(f.position() - 1);
            write!(f, "],")?;
        }
        // Clear the trailing comma
        f.set_position(f.position() - 1);
        write!(f, "}}")?;
        Ok(f.position())
    })();
    r.ok().and_then(|i| NonZeroUsize::new(i))
}

include!("../../patches_config.rs");
extern "C" {
    static REL_CONFIG: RelConfig;
}

// Based on
// https://github.com/AxioDL/PWEQuickplayPatch/blob/249ae82cc20031fe99894524aefb1f151430bedf/Source/QuickplayModule.cpp#L150
#[patch_fn(kind = "call",
           target = "OnMessage__9CMainFlowFRC20CArchitectureMessageR18CArchitectureQueue" + 72)]
unsafe extern "C" fn quickplay_hook_advance_game_state(
    flow: *mut CMainFlow,
    q: *mut CArchitectureQueue
)
{
    static mut INIT: bool = false;
    if CMainFlow::game_state(flow) == CMainFlow::CLIENT_FLOW_STATE_PRE_FRONT_END  && !INIT {
        INIT = true;
        if REL_CONFIG.quickplay_mlvl != 0xFFFFFFFF {
            let game_state = CGameState::global_instance();
            CGameState::set_current_world_id(game_state, REL_CONFIG.quickplay_mlvl);
            let world_state = CGameState::get_current_world_state(game_state);
            CWorldState::set_desired_area_asset_id(world_state, REL_CONFIG.quickplay_mrea);
            CMainFlow::set_game_state(flow, CMainFlow::CLIENT_FLOW_STATE_GAME, q);
            return;
        }
    }
    CMainFlow::advance_game_state(flow, q)
}

