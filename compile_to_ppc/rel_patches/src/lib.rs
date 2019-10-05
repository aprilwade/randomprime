#![feature(associated_type_bounds)]
#![feature(macros_in_extern)]
#![feature(never_type)]
#![feature(try_blocks)]
#![feature(type_alias_impl_trait)]
#![no_std]

extern crate alloc;

use linkme::distributed_slice;

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
use core::pin::Pin;
use core::task::{Context, Poll};
use core::writeln;

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

static mut EVENT_LOOP: Option<Pin<Box<dyn Future<Output = !>>>> = None;

#[prolog_fn]
unsafe extern "C" fn setup_global_state()
{
    debug_assert!(EVENT_LOOP.is_none());
    EVENT_LOOP = Some(Pin::new_unchecked(Box::new(event_loop())));
    primeapi::printf(b"EVENT_LOOP size: %d\n\0".as_ptr(), core::mem::size_of_val(&event_loop()));
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

    let event_loop = EVENT_LOOP.as_mut().unwrap();
    let waker = async_utils::empty_waker();
    let mut ctx = Context::from_waker(&waker);
    match event_loop.as_mut().poll(&mut ctx) {
        Poll::Pending => (),
        Poll::Ready(never) => never,
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

async fn event_loop() -> !
{
    let ss = sock_async::SockSystem::new().await;
    let addr = sock_async::SockAddr {
        len: 8,
        family: sock_async::AF_INET as u8,
        port: 9112,
        name: sock_async::INADDR_ANY,
        unused: Default::default(),
    };
    let mut server = ss.tcp_listen(&addr, 1).await.unwrap();
    loop {
        let mut client = server.accept().await.unwrap();

        let (send, recv) = client.split();
        let mut buf = Aligned32::new([MaybeUninit::uninit(); 4096]);
        match crate::http::handle_http_request(&mut buf[..], recv, send).await {
            Ok(_) => unsafe { primeapi::printf(b"successful http request\n\0".as_ptr()); },
            Err(crate::http::HttpRequestError::ReaderIO(e)) => {
                let _ = writeln!(primeapi::Mp1Stdout, "Reader IO Error: {:?}", e);
            },
            Err(crate::http::HttpRequestError::WriterIO(e)) => {
                let _ = writeln!(primeapi::Mp1Stdout, "Writer IO Error: {:?}", e);
            },
            Err(crate::http::HttpRequestError::Internal) => unsafe {
                primeapi::printf(b"failed http request\n\0".as_ptr());
            },
        }

        // loop {

        //     let game_state = CGameState::global_instance();
        //     if game_state.is_null() {
        //         async_utils::stall_once().await;
        //         continue
        //     }

        //     let player_state_ptr = unsafe { CGameState::player_state(game_state) };
        //     if player_state_ptr.is_null() {
        //         async_utils::stall_once().await;
        //         continue
        //     }
        //     let player_state = unsafe { ptr::read(player_state_ptr) };
        //     if player_state.is_null() {
        //         async_utils::stall_once().await;
        //         continue
        //     }

        //     use generic_array::{GenericArray, typenum::U4096};
        //     let mut buf = Aligned32::new(GenericArray::<u8, U4096>::default());

        //     let res: crate::sock_async::Result<()> = try {
        //         let len = build_tracker_data_json(&mut buf, game_state, player_state)?;
        //         client.send(buf.as_inner_slice().truncate_to_len(len)).await?;
        //     };
        //     if res.is_err() {
        //         break
        //     }

        //     delay(milliseconds_to_ticks(5000)).await;
        // }
    }
}

const PICKUP_MEMORY_RELAY_IDS: &[(u32, &[(u32, u32)])] = &[
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


#[allow(unused)]
fn build_tracker_data_json(
    buf: &mut[u8],
    game_state: *mut CGameState,
    player_state: *mut CPlayerState,
) -> crate::sock_async::Result<usize>
{
    let mut f = FmtCursor::new(&mut buf[..]);
    (|| -> Result<_, core::fmt::Error> {
        // TODO: Using rust's builti-n formating on a float costs ~20k of file size :(
        //       Can we work around with sprintf or something manual?
        //       OR: Just report play time in milliseconds (ie as an int!)
        write!(f, "{{\"play_time\":{:.1},\"inventory\":[",
            unsafe {CGameState::play_time(game_state) }
        )?;

        for i in 0..41 {
            let cap = unsafe { CPlayerState::get_item_capacity(player_state, i) };
            write!(f, "{},", cap)?;
        }
        // Clear the trailing comma
        f.set_position(f.position() - 1);
        write!(f, "],\"pickup_locations\":[")?;

        let mut pickup_locations = [false; 100];

        // Its _probably_ safe to use a ref here, right?
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
                        pickup_locations[*loc_idx as usize] = true;
                    }
                }
            }
        }

        for collected in pickup_locations.iter() {
            write!(f, "{},", *collected as u8)?;
        }
        f.set_position(f.position() - 1);
        write!(f, "]}}\n")?;
        Ok(())
    })().map_err(|_| crate::sock_async::Error::RANDOMPRIME)?;

    Ok(f.position())
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

