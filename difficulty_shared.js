"use strict";

var DifficultyShared = (function() {
    function item_req(item, count)
    {
        return {
            kind: 'unit',
            item: item,
            count: count,
        };
    }

    // Decompose to disjunctive normal form at each step.

    function all(...items)
    {
        var child_lists = [[]];
        items.forEach(function(item) {
            // If we have an int, convert it to an object
            if(typeof item == 'number') {
                item = item_req(item, 1);
            }

            if(item.kind == 'unit') {
                child_lists.forEach(function(list) {
                    list.push(item);
                });
            } else if(item.kind == 'disjunct') {
                var new_children = [];
                child_lists.forEach(function(list) {
                    item.conjuncts.forEach(function(conjunct) {
                        new_children.push(conjunct.concat(list));
                    });
                });
                child_lists = new_children;
            }
        });

        return {
            kind: 'disjunct',
            conjuncts: child_lists,
        };
    }

    //struct Conjunction(Vec<(u32, u32)>);
    //struct Disjunction(Vec<Conjunction>);

    function any(...items)
    {
        var child_lists = [];
        items.forEach(function(item) {
            if(typeof item == 'number') {
                item = item_req(item, 1);
            }

            if(item.kind == 'unit') {
                child_lists.push([item]);
            } else if(item.kind == 'disjunct') {
                item.conjuncts.forEach(function(conjunct) {
                    child_lists.push(conjunct);
                });
            }
        });

        return {
            kind: 'disjunct',
            conjuncts: child_lists,
        };
    }

    function n_of(item, n)
    {
        return item_req(item, n);
    }


    var MISSILE = 0;
    var ENERGY_TANK = 1;

    var THERMAL_VISOR = 2;
    var XRAY_VISOR = 3;

    var VARIA_SUIT = 4;
    var GRAVITY_SUIT = 5;
    var PHAZON_SUIT = 6;

    var MORPH_BALL = 7;
    var BOOST_BALL = 8;
    var SPIDER_BALL = 9;

    var MORPH_BALL_BOMB = 10;
    var POWER_BOMB_EXPANSION = 11;
    var POWER_BOMB = 12;

    var CHARGE_BEAM = 13;
    var SPACE_JUMP_BOOTS = 14;
    var GRAPPLE_BEAM = 15;

    var SUPER_MISSILE = 16;
    var WAVEBUSTER = 17;
    var ICE_SPREADER = 18;
    var FLAMETHROWER = 19;

    var WAVE_BEAM = 20;
    var ICE_BEAM = 21;
    var PLASMA_BEAM = 22;

    var ARTIFACT_LIFEGIVER = 23;
    var ARTIFACT_WILD = 24;
    var ARTIFACT_WORLD = 25;
    var ARTIFACT_SUN = 26;
    var ARTIFACT_ELDER = 27;
    var ARTIFACT_SPIRIT = 28;
    var ARTIFACT_TRUTH = 29;
    var ARTIFACT_CHOZO = 30;
    var ARTIFACT_WARRIOR = 31;
    var ARTIFACT_NEWBORN = 32;
    var ARTIFACT_NATURE = 33;
    var ARTIFACT_STRENGTH = 34;

    var NOTHING = 35;


    var ANY_SUIT = any(VARIA_SUIT, GRAVITY_SUIT, PHAZON_SUIT);
    var ANY_POWER_BOMBS = any(POWER_BOMB, POWER_BOMB_EXPANSION);

    var MBB_OR_PB = any(MORPH_BALL_BOMB, POWER_BOMB);

    /*
    var ALL_ITEMS = new Map([
        [MISSILE, 50],
        [ENERGY_TANK, 14],

        [THERMAL_VISOR, 1],
        [XRAY_VISOR, 1],

        [VARIA_SUIT, 1],
        [GRAVITY_SUIT, 1],
        [PHAZON_SUIT, 1],

        [MORPH_BALL, 1],
        [BOOST_BALL, 1],
        [SPIDER_BALL, 1],

        [MORPH_BALL_BOMB, 1],
        [POWER_BOMB_EXPANSION, 4],
        [POWER_BOMB, 1],

        [CHARGE_BEAM, 1],
        [SPACE_JUMP_BOOTS, 1],
        [GRAPPLE_BEAM, 1],

        [SUPER_MISSILE, 1],
        [WAVEBUSTER, 1],
        [ICE_SPREADER, 1],
        [FLAMETHROWER, 1],

        [WAVE_BEAM, 1],
        [ICE_BEAM, 1],
        [PLASMA_BEAM, 1],

        [ARTIFACT_LIFEGIVER, 1],
        [ARTIFACT_WILD, 1],
        [ARTIFACT_WORLD, 1],
        [ARTIFACT_SUN, 1],
        [ARTIFACT_ELDER, 1],
        [ARTIFACT_SPIRIT, 1],
        [ARTIFACT_TRUTH, 1],
        [ARTIFACT_CHOZO, 1],
        [ARTIFACT_WARRIOR, 1],
        [ARTIFACT_NEWBORN, 1],
        [ARTIFACT_NATURE, 1],
        [ARTIFACT_STRENGTH, 1],
    ]);
    */

    return {
        all: all,
        any: any,
        n_of: n_of,
        MISSILE: MISSILE,
        ENERGY_TANK: ENERGY_TANK,

        THERMAL_VISOR: THERMAL_VISOR,
        XRAY_VISOR: XRAY_VISOR,

        VARIA_SUIT: VARIA_SUIT,
        GRAVITY_SUIT: GRAVITY_SUIT,
        PHAZON_SUIT: PHAZON_SUIT,

        MORPH_BALL: MORPH_BALL,
        BOOST_BALL: BOOST_BALL,
        SPIDER_BALL: SPIDER_BALL,

        MORPH_BALL_BOMB: MORPH_BALL_BOMB,
        POWER_BOMB_EXPANSION: POWER_BOMB_EXPANSION,
        POWER_BOMB: POWER_BOMB,

        CHARGE_BEAM: CHARGE_BEAM,
        SPACE_JUMP_BOOTS: SPACE_JUMP_BOOTS,
        GRAPPLE_BEAM: GRAPPLE_BEAM,

        SUPER_MISSILE: SUPER_MISSILE,
        WAVEBUSTER: WAVEBUSTER,
        ICE_SPREADER: ICE_SPREADER,
        FLAMETHROWER: FLAMETHROWER,

        WAVE_BEAM: WAVE_BEAM,
        ICE_BEAM: ICE_BEAM,
        PLASMA_BEAM: PLASMA_BEAM,

        ARTIFACT_LIFEGIVER: ARTIFACT_LIFEGIVER,
        ARTIFACT_WILD: ARTIFACT_WILD,
        ARTIFACT_WORLD: ARTIFACT_WORLD,
        ARTIFACT_SUN: ARTIFACT_SUN,
        ARTIFACT_ELDER: ARTIFACT_ELDER,
        ARTIFACT_SPIRIT: ARTIFACT_SPIRIT,
        ARTIFACT_TRUTH: ARTIFACT_TRUTH,
        ARTIFACT_CHOZO: ARTIFACT_CHOZO,
        ARTIFACT_WARRIOR: ARTIFACT_WARRIOR,
        ARTIFACT_NEWBORN: ARTIFACT_NEWBORN,
        ARTIFACT_NATURE: ARTIFACT_NATURE,
        ARTIFACT_STRENGTH: ARTIFACT_STRENGTH,

        NOTHING: NOTHING,

        ANY_SUIT: ANY_SUIT,
        ANY_POWER_BOMBS: ANY_POWER_BOMBS,
        MBB_OR_PB: MBB_OR_PB,

    };
}());
