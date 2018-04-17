"use strict";

var NormalDifficulty = (function() {
    var all = DifficultyShared.all;
    var any = DifficultyShared.any;
    var n_of = DifficultyShared.n_of;
    var MISSILE = DifficultyShared.MISSILE;
    var ENERGY_TANK = DifficultyShared.ENERGY_TANK;

    var THERMAL_VISOR = DifficultyShared.THERMAL_VISOR;
    var XRAY_VISOR = DifficultyShared.XRAY_VISOR;

    var VARIA_SUIT = DifficultyShared.VARIA_SUIT;
    var GRAVITY_SUIT = DifficultyShared.GRAVITY_SUIT;
    var PHAZON_SUIT = DifficultyShared.PHAZON_SUIT;

    var MORPH_BALL = DifficultyShared.MORPH_BALL;
    var BOOST_BALL = all(MORPH_BALL, DifficultyShared.BOOST_BALL);
    var SPIDER_BALL = all(MORPH_BALL, DifficultyShared.SPIDER_BALL);

    var MORPH_BALL_BOMB = all(MORPH_BALL, DifficultyShared.MORPH_BALL_BOMB);
    var POWER_BOMB_EXPANSION = all(MORPH_BALL, DifficultyShared.POWER_BOMB_EXPANSION);
    var POWER_BOMB = all(MORPH_BALL, DifficultyShared.POWER_BOMB);

    var CHARGE_BEAM = DifficultyShared.CHARGE_BEAM;
    var SPACE_JUMP_BOOTS = DifficultyShared.SPACE_JUMP_BOOTS;
    var GRAPPLE_BEAM = DifficultyShared.GRAPPLE_BEAM;

    var SUPER_MISSILE = all(CHARGE_BEAM, MISSILE, DifficultyShared.SUPER_MISSILE);
    var WAVEBUSTER = DifficultyShared.WAVEBUSTER;
    var ICE_SPREADER = DifficultyShared.ICE_SPREADER;
    var FLAMETHROWER = DifficultyShared.FLAMETHROWER;

    var WAVE_BEAM = DifficultyShared.WAVE_BEAM;
    var ICE_BEAM = DifficultyShared.ICE_BEAM;
    var PLASMA_BEAM = DifficultyShared.PLASMA_BEAM;

    var ANY_SUIT = DifficultyShared.ANY_SUIT;
    var ANY_POWER_BOMBS = DifficultyShared.ANY_POWER_BOMBS;
    var MBB_OR_PB = DifficultyShared.MBB_OR_PB;

    var PHENDRANA_REQS = all(MISSILE, MORPH_BALL_BOMB, ANY_SUIT);
    // XXX With BBJ + unmore this doesn't require SJB
    var BACKWARDS_PHENDRANA_REQS = all(MISSILE, ANY_SUIT, SPIDER_BALL, SPACE_JUMP_BOOTS,
                                       WAVE_BEAM);
    var FRIGATE_REQS = all(MISSILE, MORPH_BALL, WAVE_BEAM, THERMAL_VISOR, GRAVITY_SUIT);
    var MINES_FROM_TALLON_REQS = all(MISSILE, MORPH_BALL_BOMB, SPACE_JUMP_BOOTS, GRAVITY_SUIT,
                                     THERMAL_VISOR, WAVE_BEAM, ICE_BEAM);
    var MINES_FROM_MAGMOOR_REQS = all(MISSILE, SPIDER_BALL, SPACE_JUMP_BOOTS, ANY_SUIT, WAVE_BEAM,
                                      ICE_BEAM, ANY_POWER_BOMBS);

    return {
        // TODO: Consider replacing optional items with required items
        'optional_items': [
            [DifficultyShared.MISSILE, 49],
            [DifficultyShared.ENERGY_TANK, 14],
            [DifficultyShared.VARIA_SUIT, 1],
            [DifficultyShared.POWER_BOMB, 1],
            [DifficultyShared.POWER_BOMB_EXPANSION, 3],
            [DifficultyShared.WAVEBUSTER, 1],
            [DifficultyShared.ICE_SPREADER, 1],
            [DifficultyShared.FLAMETHROWER, 1],
        ],
        'room_reqs': [
            // 0: Chozo - - - Main Plaza (Half-Pipe) - - - - - - - - Missile Expansion 1
            {
                required: all(BOOST_BALL),
            },

            // 1: Chozo - - - Main Plaza (Grapple Ledge) - - - - - - Missile Expansion 2
            {
                required: all(MISSILE, GRAPPLE_BEAM, ANY_SUIT, BOOST_BALL, MORPH_BALL_BOMB,
                              WAVE_BEAM),
            },

            // 2: Chozo - - - Main Plaza (Tree)  - - - - - - - - - - Missile Expansion 3
            {
                required: all(SUPER_MISSILE),
            },

            // 3: Chozo - - - Main Plaza (Locked Door) - - - - - - - Energy Tank 1
            {
                required: all(MISSILE, MORPH_BALL_BOMB),
            },

            // 4: Chozo - - - Ruined Fountain  - - - - - - - - - - - Missile Expansion 4
            {
                required: all(MISSILE, SPIDER_BALL),
            },

            // 5: Chozo - - - Ruined Shrine ("Beetle Battle")  - - - Morph Ball
            {
                required: all(MISSILE),
                escape: any(SPACE_JUMP_BOOTS, MORPH_BALL),
            },

            // 6: Chozo - - - Ruined Shrine (Half-Pipe)  - - - - - - Missile Expansion 5
            {
                required: all(MISSILE, BOOST_BALL),
            },

            // 7: Chozo - - - Ruined Shrine (Lower Tunnel) - - - - - Missile Expansion 6
            {
                required: all(MISSILE, MBB_OR_PB),
            },

            // 8: Chozo - - - Vault  - - - - - - - - - - - - - - - - Missile Expansion 7
            {
                required: all(MISSILE, MORPH_BALL_BOMB),
            },

            // 9: Chozo - - - Training Chamber - - - - - - - - - - - Energy Tank 2
            {
                required: all(MISSILE, ANY_SUIT, GRAPPLE_BEAM, WAVE_BEAM, BOOST_BALL,
                              SPIDER_BALL),
            },

            //10: Chozo - - - Ruined Nursery - - - - - - - - - - - - Missile Expansion 8
            {
                required: all(MORPH_BALL_BOMB),
            },

            //11: Chozo - - - Training Chamber Access  - - - - - - - Missile Expansion 9
            {
                required: all(MISSILE, ANY_SUIT, GRAPPLE_BEAM, WAVE_BEAM, MORPH_BALL),
            },

            //12: Chozo - - - Magma Pool - - - - - - - - - - - - - - Power Bomb Expansion 1
            {
                required: all(MISSILE, ANY_SUIT, GRAPPLE_BEAM, ANY_POWER_BOMBS),
            },

            //13: Chozo - - - Tower of Light - - - - - - - - - - - - Wavebuster
            {
                // This actually only requires 75 missiles, but 80 is less hard
                required: all(n_of(MISSILE, 80 / 5), BOOST_BALL, SPIDER_BALL, WAVE_BEAM,
                                SPACE_JUMP_BOOTS),
            },

            //14: Chozo - - - Tower Chamber  - - - - - - - - - - - - Artifact of Lifegiver
            {
                // XXX Requires SJB without GS, but w/ GS doesn't
                required: all(MISSILE, BOOST_BALL, SPIDER_BALL, WAVE_BEAM, GRAVITY_SUIT,
                              SPACE_JUMP_BOOTS),
            },

            //15: Chozo - - - Ruined Gallery (Missile Wall)  - - - - Missile Expansion 10
            {
                required: all(MISSILE),
            },

            //16: Chozo - - - Ruined Gallery (Tunnel)  - - - - - - - Missile Expansion 11
            {
                required: all(MORPH_BALL_BOMB),
            },

            //17: Chozo - - - Transport Access North - - - - - - - - Energy Tank 3
            {
                required: all(MISSILE),
            },

            //18: Chozo - - - Gathering Hall - - - - - - - - - - - - Missile Expansion 12
            {
                required: all(MISSILE, MBB_OR_PB, SPACE_JUMP_BOOTS),
            },

            //19: Chozo - - - Hive Totem - - - - - - - - - - - - - - Missile Launcher
            {
                required: all(),
            },

            //20: Chozo - - - Sunchamber (Flaahgra)  - - - - - - - - Varia Suit
            {
                required: all(MISSILE, MORPH_BALL_BOMB),
            },

            //21: Chozo - - - Sunchamber (Ghosts)  - - - - - - - - - Artifact of Wild
            {
                // XXX MBB is very questionable here...
                //     Its not strickly needed, but it is needed to fight flaahgra
                required: all(MORPH_BALL_BOMB, SPIDER_BALL, SUPER_MISSILE),
            },

            //22: Chozo - - - Watery Hall Access - - - - - - - - - - Missile Expansion 13
            {
                required: all(MISSILE, MORPH_BALL),
            },

            //23: Chozo - - - Watery Hall (Scan Puzzle)  - - - - - - Charge Beam
            {
                required: all(MISSILE, MORPH_BALL),
            },

            //24: Chozo - - - Watery Hall (Underwater) - - - - - - - Missile Expansion 14
            {
                // NOTE: Does this actually require Space Jump? (Yes, without a dbj + unmorph)
                required: all(MISSILE, MORPH_BALL_BOMB, GRAVITY_SUIT, SPACE_JUMP_BOOTS),
            },

            //25: Chozo - - - Dynamo (Lower) - - - - - - - - - - - - Missile Expansion 15
            {
                required: all(MISSILE, MBB_OR_PB),
            },

            //26: Chozo - - - Dynamo (Spider Track)  - - - - - - - - Missile Expansion 16
            {
                required: all(MISSILE, MBB_OR_PB, SPIDER_BALL),
            },

            //27: Chozo - - - Burn Dome (Missile)  - - - - - - - - - Missile Expansion 17
            {
                required: all(MISSILE, MBB_OR_PB),
                escape: all(MORPH_BALL_BOMB),
            },

            //28: Chozo - - - Burn Dome (I. Drone) - - - - - - - - - Morph Ball Bomb
            {
                required: all(MISSILE, MORPH_BALL),
                escape: all(MORPH_BALL_BOMB),
            },

            //29: Chozo - - - Furnace (Spider Tracks)  - - - - - - - Missile Expansion 18
            {
                required: all(MISSILE, MORPH_BALL_BOMB, ANY_POWER_BOMBS, BOOST_BALL,
                              SPIDER_BALL),
            },

            //30: Chozo - - - Furnace (Inside Furnace) - - - - - - - Energy Tank 4
            {
                required: all(MISSILE, MORPH_BALL_BOMB),
            },

            //31: Chozo - - - Hall of the Elders - - - - - - - - - - Energy Tank 5
            {
                required: all(MISSILE, MORPH_BALL_BOMB, SPIDER_BALL, WAVE_BEAM, BOOST_BALL,
                              ICE_BEAM, SPACE_JUMP_BOOTS),
            },

            //32: Chozo - - - Crossway - - - - - - - - - - - - - - - Missile Expansion 19
            {
                required: all(MISSILE, MORPH_BALL_BOMB, SPIDER_BALL, WAVE_BEAM, BOOST_BALL),
            },

            //33: Chozo - - - Elder Chamber  - - - - - - - - - - - - Artifact of World
            {
                required: all(MISSILE, MORPH_BALL_BOMB, SPIDER_BALL, WAVE_BEAM, BOOST_BALL,
                              PLASMA_BEAM, SPACE_JUMP_BOOTS),
                escape: all(ICE_BEAM),
            },

            //34: Chozo - - - Antechamber  - - - - - - - - - - - - - Ice Beam
            {
                required: all(MISSILE, MORPH_BALL_BOMB, SPIDER_BALL, WAVE_BEAM, BOOST_BALL,
                              SPACE_JUMP_BOOTS),
                escape: all(ICE_BEAM),
            },

            //35: Phendrana - Phendrana Shorelines (Behind Ice)  - - Missile Expansion 20
            {
                required: all(PHENDRANA_REQS, PLASMA_BEAM),
            },

            //36: Phendrana - Phendrana Shorelines (Spider Track)  - Missile Expansion 21
            {
                required: all(PHENDRANA_REQS, SPACE_JUMP_BOOTS, SPIDER_BALL, SUPER_MISSILE),
            },

            //37: Phendrana - Chozo Ice Temple - - - - - - - - - - - Artifact of Sun
            {
                required: all(PHENDRANA_REQS, SPACE_JUMP_BOOTS, PLASMA_BEAM),
            },

            //38: Phendrana - Ice Ruins West - - - - - - - - - - - - Power Bomb Expansion 2
            {
                required: all(PHENDRANA_REQS, PLASMA_BEAM, SPACE_JUMP_BOOTS),
            },

            //39: Phendrana - Ice Ruins East (Behind Ice)  - - - - - Missile Expansion 22
            {
                required: all(PHENDRANA_REQS, PLASMA_BEAM),
            },

            //40: Phendrana - Ice Ruins East (Spider Track)  - - - - Missile Expansion 23
            {
                required: all(PHENDRANA_REQS, SPIDER_BALL),
            },

            //41: Phendrana - Chapel of the Elders - - - - - - - - - Wave Beam
            {
                required: all(PHENDRANA_REQS, SPACE_JUMP_BOOTS),
                escape: all(WAVE_BEAM),
            },

            //42: Phendrana - Ruined Courtyard - - - - - - - - - - - Energy Tank 6
            {
                required: all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM),
            },

            //43: Phendrana - Phendrana Canyon - - - - - - - - - - - Boost Ball
            {
                required: all(PHENDRANA_REQS),
                // XXX Strictly speaking, you can escape without either of
                // these, but it requires jumping on destructable boxes, and
                // thus makes this room a potential hazard
                escape: any(BOOST_BALL, SPACE_JUMP_BOOTS),
            },

            //44: Phendrana - Quarantine Cave  - - - - - - - - - - - Spider Ball
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, THERMAL_VISOR),
                    // XXX Thermal visor is only required for fighting Thardus. It could be removed
                    all(BACKWARDS_PHENDRANA_REQS,  THERMAL_VISOR)
                ),
                escape: all(SPIDER_BALL),
            },

            //45: Phendrana - Research Lab Hydra - - - - - - - - - - Missile Expansion 24
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, THERMAL_VISOR,
                        SUPER_MISSILE),
                    // XXX Research Core Thermal Visor
                    all(BACKWARDS_PHENDRANA_REQS, THERMAL_VISOR, ICE_BEAM, SUPER_MISSILE)
                ),
            },

            //46: Phendrana - Quarantine Monitor - - - - - - - - - - Missile Expansion 25
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, THERMAL_VISOR,
                        GRAPPLE_BEAM),
                    // XXX Thermal visor is only required for fighting Thardus. It could be removed?
                    all(BACKWARDS_PHENDRANA_REQS,  THERMAL_VISOR, GRAPPLE_BEAM)
                ),
                escape: all(SPIDER_BALL),
            },

            //47: Phendrana - Observatory  - - - - - - - - - - - - - Super Missile
            {
                required: all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM),
            },

            //48: Phendrana - Transport Access - - - - - - - - - - - Energy Tank 7
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, THERMAL_VISOR,
                        SPIDER_BALL, PLASMA_BEAM),
                    // XXX Research Core Thermal Visor
                    all(BACKWARDS_PHENDRANA_REQS, THERMAL_VISOR, ICE_BEAM, PLASMA_BEAM)
                ),
            },

            //49: Phendrana - Control Tower  - - - - - - - - - - - - Artifact of Elder
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, PLASMA_BEAM),
                    // XXX Research Core Thermal Visor
                    all(BACKWARDS_PHENDRANA_REQS, THERMAL_VISOR, ICE_BEAM, PLASMA_BEAM)
                ),
            },

            //50: Phendrana - Research Core  - - - - - - - - - - - - Thermal Visor
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM),
                    all(BACKWARDS_PHENDRANA_REQS, ICE_BEAM)
                ),
                escape: any(THERMAL_VISOR, ICE_BEAM),
            },

            //51: Phendrana - Frost Cave - - - - - - - - - - - - - - Missile Expansion 26
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, ICE_BEAM,
                        GRAPPLE_BEAM),
                    all(BACKWARDS_PHENDRANA_REQS, ICE_BEAM, GRAPPLE_BEAM)
                ),
                // The thermal visor is required to escape either via Research
                // Core or fight Thardus
                escape: any(THERMAL_VISOR, BACKWARDS_PHENDRANA_REQS),
            },

            //52: Phendrana - Research Lab Aether (Tank) - - - - - - Energy Tank 8
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM),
                    // XXX Research Core Thermal Visor
                    all(BACKWARDS_PHENDRANA_REQS, THERMAL_VISOR, ICE_BEAM)
                ),
            },

            //53: Phendrana - Research Lab Aether (Morph Track)  - - Missile Expansion 27
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM),
                    // XXX Research Core Thermal Visor
                    all(BACKWARDS_PHENDRANA_REQS, THERMAL_VISOR, ICE_BEAM)
                ),
            },

            //54: Phendrana - Gravity Chamber (Underwater) - - - - - Gravity Suit
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, ICE_BEAM),
                    all(BACKWARDS_PHENDRANA_REQS, ICE_BEAM)
                ),
                escape: all(GRAVITY_SUIT, any(THERMAL_VISOR, BACKWARDS_PHENDRANA_REQS)),
            },

            //55: Phendrana - Gravity Chamber (Grapple Ledge)  - - - Missile Expansion 28
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, ICE_BEAM,
                        PLASMA_BEAM, GRAPPLE_BEAM, GRAVITY_SUIT),
                    all(BACKWARDS_PHENDRANA_REQS, ICE_BEAM, PLASMA_BEAM, GRAPPLE_BEAM, GRAVITY_SUIT)
                ),
                // See 51
                escape: any(THERMAL_VISOR, BACKWARDS_PHENDRANA_REQS),
            },

            //56: Phendrana - Storage Cave - - - - - - - - - - - - - Artifact of Spirit
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, ICE_BEAM,
                        THERMAL_VISOR, PLASMA_BEAM, POWER_BOMB, GRAPPLE_BEAM),
                    all(BACKWARDS_PHENDRANA_REQS, ICE_BEAM, PLASMA_BEAM, POWER_BOMB, GRAPPLE_BEAM)
                ),
                // See 51
                escape: any(THERMAL_VISOR, BACKWARDS_PHENDRANA_REQS),
            },

            //57: Phendrana - Security Cave  - - - - - - - - - - - - Power Bomb Expansion 3
            {
                required: any(
                    all(PHENDRANA_REQS, BOOST_BALL, SPACE_JUMP_BOOTS, WAVE_BEAM, ICE_BEAM,
                        GRAPPLE_BEAM),
                    all(BACKWARDS_PHENDRANA_REQS, ICE_BEAM, GRAPPLE_BEAM)
                ),
                // See 51
                escape: any(THERMAL_VISOR, BACKWARDS_PHENDRANA_REQS),
            },

            //58: Tallon  - - Landing Site - - - - - - - - - - - - - Missile Expansion 29
            {
                required: all(MORPH_BALL),
            },

            //59: Tallon  - - Alcove - - - - - - - - - - - - - - - - Space Jump Boots
            {
                required: any(all(MORPH_BALL_BOMB, BOOST_BALL),
                              SPACE_JUMP_BOOTS),
            },

            //60: Tallon  - - Frigate Crash Site - - - - - - - - - - Missile Expansion 30
            {
                required: all(MISSILE, MORPH_BALL, GRAVITY_SUIT, SPACE_JUMP_BOOTS),
            },

            //61: Tallon  - - Overgrown Cavern - - - - - - - - - - - Missile Expansion 31
            {
                required: all(MISSILE, MORPH_BALL_BOMB, WAVE_BEAM, SPIDER_BALL, BOOST_BALL,
                              ICE_BEAM, SPACE_JUMP_BOOTS),
            },

            //62: Tallon  - - Root Cave  - - - - - - - - - - - - - - Missile Expansion 32
            {
                required: all(MISSILE, SPACE_JUMP_BOOTS, GRAPPLE_BEAM, XRAY_VISOR),
            },

            //63: Tallon  - - Artifact Temple  - - - - - - - - - - - Artifact of Truth
            {
                required: all(MISSILE),
            },

            //64: Tallon  - - Transport Tunnel B - - - - - - - - - - Missile Expansion 33
            {
                required: all(MISSILE),
            },

            //65: Tallon  - - Arbor Chamber  - - - - - - - - - - - - Missile Expansion 34
            {
                required: all(MISSILE, SPACE_JUMP_BOOTS, GRAPPLE_BEAM, XRAY_VISOR, PLASMA_BEAM),
            },

            //66: Tallon  - - Cargo Freight Lift to Deck Gamma - - - Energy Tank 9
            {
                required: all(MISSILE, MORPH_BALL, GRAVITY_SUIT, THERMAL_VISOR, WAVE_BEAM,
                              ICE_BEAM),
            },

            //67: Tallon  - - Biohazard Containment  - - - - - - - - Missile Expansion 35
            {
                required: any(
                    all(MORPH_BALL, GRAVITY_SUIT, THERMAL_VISOR, WAVE_BEAM, ICE_BEAM,
                        SPACE_JUMP_BOOTS, SUPER_MISSILE),
                    // Backwards through Phazon Mines. Requires this to contain the thermal visor.
                    all(MISSILE, MORPH_BALL, GRAVITY_SUIT, SPIDER_BALL, SPACE_JUMP_BOOTS,
                        WAVE_BEAM, ICE_BEAM, ANY_POWER_BOMBS, GRAPPLE_BEAM)
                ),
                escape: any(THERMAL_VISOR),
            },

            //68: Tallon  - - Hydro Access Tunnel  - - - - - - - - - Energy Tank 10
            {
                required: any(
                    all(MISSILE, MORPH_BALL, GRAVITY_SUIT, THERMAL_VISOR, WAVE_BEAM, ICE_BEAM,
                        SPACE_JUMP_BOOTS, MORPH_BALL_BOMB),
                    // Backwards through Phazon Mines
                    // TODO Is the Grapple Beam requirement necessary? A simple l-jump
                    //      by passes it easily.
                    all(MISSILE, MORPH_BALL, GRAVITY_SUIT, SPIDER_BALL, SPACE_JUMP_BOOTS,
                        WAVE_BEAM, ICE_BEAM, ANY_POWER_BOMBS, GRAPPLE_BEAM)
                ),
            },

            //69: Tallon  - - Great Tree Chamber - - - - - - - - - - Missile Expansion 36
            {
                required: any(
                    // From frigate
                    all(MISSILE, MORPH_BALL_BOMB, WAVE_BEAM, ICE_BEAM, SPACE_JUMP_BOOTS,
                        XRAY_VISOR, GRAVITY_SUIT, THERMAL_VISOR),
                    // From backwards mines
                    all(MISSILE, MORPH_BALL_BOMB, WAVE_BEAM, ICE_BEAM, SPACE_JUMP_BOOTS,
                        XRAY_VISOR, ANY_SUIT, SPIDER_BALL, ANY_POWER_BOMBS, GRAPPLE_BEAM),
                    // From chozo
                    all(MISSILE, MORPH_BALL_BOMB, WAVE_BEAM, ICE_BEAM, SPACE_JUMP_BOOTS,
                        XRAY_VISOR, SPIDER_BALL, BOOST_BALL)
                ),
            },

            //70: Tallon  - - Life Grove Tunnel  - - - - - - - - - - Missile Expansion 37
            {
                required: all(MISSILE, GRAVITY_SUIT, WAVE_BEAM, ICE_BEAM, SPACE_JUMP_BOOTS,
                              MORPH_BALL_BOMB, SPIDER_BALL, BOOST_BALL, ANY_POWER_BOMBS),
            },

            //71: Tallon  - - Life Grove (Start) - - - - - - - - - - X-Ray Visor
            {
                required: all(MISSILE, GRAVITY_SUIT, WAVE_BEAM, ICE_BEAM, SPACE_JUMP_BOOTS,
                              MORPH_BALL_BOMB, SPIDER_BALL, BOOST_BALL, ANY_POWER_BOMBS),
            },

            //72: Tallon  - - Life Grove (Underwater Spinner)  - - - Artifact of Chozo
            {
                // XXX Gravity suit: Its not actually required, but could be considered a glitch.
                required: all(MISSILE, GRAVITY_SUIT, WAVE_BEAM, ICE_BEAM, SPACE_JUMP_BOOTS,
                              MORPH_BALL_BOMB, SPIDER_BALL, BOOST_BALL, ANY_POWER_BOMBS),
            },

            //73: Mines - - - Main Quarry  - - - - - - - - - - - - - Missile Expansion 38
            {
                required: all(MISSILE, ANY_SUIT, WAVE_BEAM, ICE_BEAM, SPACE_JUMP_BOOTS,
                              MORPH_BALL_BOMB, SPIDER_BALL, THERMAL_VISOR),
            },

            //74: Mines - - - Security Access A  - - - - - - - - - - Missile Expansion 39
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL),
                    all(MINES_FROM_MAGMOOR_REQS, MORPH_BALL_BOMB)
                ),
            },

            //75: Mines - - - Storage Depot B  - - - - - - - - - - - Grapple Beam
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS),
                    all(MINES_FROM_MAGMOOR_REQS, GRAPPLE_BEAM, MORPH_BALL_BOMB)
                ),
            },

            //76: Mines - - - Storage Depot A  - - - - - - - - - - - Flamethrower
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, PLASMA_BEAM),
                    all(MINES_FROM_MAGMOOR_REQS, GRAPPLE_BEAM, MORPH_BALL_BOMB, PLASMA_BEAM)
                ),
            },

            //77: Mines - - - Elite Research (Phazon Elite)  - - - - Artifact of Warrior
            {
                // You need to #84 to unlock this fight. So, boost ball is required
                // so one can go down to it and back up.
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL),
                    all(MINES_FROM_MAGMOOR_REQS, GRAPPLE_BEAM, MORPH_BALL_BOMB, BOOST_BALL)
                ),
            },

            //78: Mines - - - Elite Research (Laser) - - - - - - - - Missile Expansion 40
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, BOOST_BALL),
                    all(MINES_FROM_MAGMOOR_REQS, GRAPPLE_BEAM, MORPH_BALL_BOMB, BOOST_BALL)
                ),
            },

            //79: Mines - - - Elite Control Access - - - - - - - - - Missile Expansion 41
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS),
                    all(MINES_FROM_MAGMOOR_REQS)
                ),
            },

            //80: Mines - - - Ventilation Shaft  - - - - - - - - - - Energy Tank 11
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, BOOST_BALL, ANY_POWER_BOMBS),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL)
                ),
            },

            //81: Mines - - - Phazon Processing Center - - - - - - - Missile Expansion 42
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS),
                    all(MINES_FROM_MAGMOOR_REQS)
                ),
            },

            //82: Mines - - - Processing Center Access - - - - - - - Energy Tank 12
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL, PLASMA_BEAM,
                        XRAY_VISOR, GRAPPLE_BEAM),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL, PLASMA_BEAM, XRAY_VISOR, GRAPPLE_BEAM)
                ),
            },

            //83: Mines - - - Elite Quarters - - - - - - - - - - - - Phazon Suit
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL, PLASMA_BEAM,
                        XRAY_VISOR),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL, PLASMA_BEAM, XRAY_VISOR)
                ),
            },

            //84: Mines - - - Central Dynamo - - - - - - - - - - - - Power Bomb
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, BOOST_BALL),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL)
                ),
            },

            //85: Mines - - - Metroid Quarantine B - - - - - - - - - Missile Expansion 43
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL, XRAY_VISOR,
                        GRAPPLE_BEAM, PLASMA_BEAM),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL, XRAY_VISOR, GRAPPLE_BEAM, PLASMA_BEAM)
                ),
            },

            //86: Mines - - - Metroid Quarantine A - - - - - - - - - Missile Expansion 44
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL, XRAY_VISOR),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL, XRAY_VISOR)
                ),
            },

            //87: Mines - - - Fungal Hall B  - - - - - - - - - - - - Missile Expansion 45
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, BOOST_BALL, XRAY_VISOR, PLASMA_BEAM),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL, XRAY_VISOR, PLASMA_BEAM)
                ),
            },

            //88: Mines - - - Phazon Mining Tunnel - - - - - - - - - Artifact of Newborn
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL, XRAY_VISOR,
                        PLASMA_BEAM, GRAPPLE_BEAM, PHAZON_SUIT),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL, XRAY_VISOR, PLASMA_BEAM, GRAPPLE_BEAM,
                        PHAZON_SUIT)
                ),
            },

            //89: Mines - - - Fungal Hall Access - - - - - - - - - - Missile Expansion 46
            {
                required: any(
                    all(MINES_FROM_TALLON_REQS, ANY_POWER_BOMBS, BOOST_BALL, XRAY_VISOR),
                    all(MINES_FROM_MAGMOOR_REQS, BOOST_BALL, XRAY_VISOR, PLASMA_BEAM)
                ),
            },

            //90: Magmoor - - Lava Lake  - - - - - - - - - - - - - - Artifact of Nature
            {
                required: all(MISSILE, MBB_OR_PB, ANY_SUIT, SPACE_JUMP_BOOTS),
            },

            //91: Magmoor - - Triclops Pit - - - - - - - - - - - - - Missile Expansion 47
            {
                // TODO Double if x-ray is needed if space jump??
                required: all(MISSILE, MORPH_BALL, ANY_SUIT, any(SPACE_JUMP_BOOTS, XRAY_VISOR)),
            },

            //92: Magmoor - - Storage Cavern - - - - - - - - - - - - Missile Expansion 48
            {
                required: all(MISSILE, MORPH_BALL, ANY_SUIT),
            },

            //93: Magmoor - - Transport Tunnel A - - - - - - - - - - Energy Tank 13
            {
                required: all(MISSILE, MORPH_BALL_BOMB, ANY_SUIT),
            },

            //94: Magmoor - - Warrior Shrine - - - - - - - - - - - - Artifact of Strength
            {
                required: all(MISSILE, ANY_SUIT, SPACE_JUMP_BOOTS, BOOST_BALL),
            },

            //95: Magmoor - - Shore Tunnel - - - - - - - - - - - - - Ice Spreader
            {
                required: all(MISSILE, ANY_POWER_BOMBS, ANY_SUIT, SPACE_JUMP_BOOTS),
            },

            //96: Magmoor - - Fiery Shores (Morph Track) - - - - - - Missile Expansion 49
            {
                // TODO This can be done using space jump, but is that too much of a glitch?
                required: all(MISSILE, MORPH_BALL_BOMB, ANY_SUIT),
            },

            //97: Magmoor - - Fiery Shores (Warrior Shrine Tunnel) - Power Bomb Expansion 4
            {
                required: all(MISSILE, ANY_SUIT, SPACE_JUMP_BOOTS, BOOST_BALL, ANY_POWER_BOMBS),
            },

            //98: Magmoor - - Plasma Processing  - - - - - - - - - - Plasma Beam
            {
                required: all(MISSILE, ANY_SUIT, SPACE_JUMP_BOOTS, BOOST_BALL, SPIDER_BALL,
                                GRAPPLE_BEAM, WAVE_BEAM, ICE_BEAM),
            },

            //99: Magmoor - - Magmoor Workstation  - - - - - - - - - Energy Tank 14
            {
                required: all(MISSILE, ANY_SUIT, SPACE_JUMP_BOOTS, SPIDER_BALL, WAVE_BEAM,
                              THERMAL_VISOR),
            },

        ],
    };
}());

