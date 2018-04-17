"use strict";

$().ready(function() {

    function generate_layout(room_requirements, max_items)
    {
        function requirements_satisfied(obtained_items, requirements)
        {
            var satisfied = false;
            for(var k = 0; !satisfied && k < requirements.length; k++) {

                satisfied = true;
                requirements[k].forEach(function (quantity, item_type) {
                    var current_quantity = obtained_items.get(item_type) || 0;
                    if(quantity > current_quantity) {
                        satisfied = false;
                    }
                });
            }

            return satisfied;
        }

        function count_missing_requirements(obtained_items, requirements)
        {
            var missing_set = new Set();
            var missing_count = 0;
            requirements.forEach(function(quantity, item_type) {
                var current_quantity = obtained_items.get(item_type) || 0;
                if(current_quantity < quantity) {
                    missing_set.add(item_type);
                    missing_count += quantity - current_quantity;
                }
            });
            return [missing_count, missing_set];
        }

        // Convert the requirements list into a more useful format
        var location_reqs = room_requirements.map(function(loc) {
            function convert(items)
            {
                var counts = new Map();
                items.forEach(function(i) {
                    counts.set(i.item, i.count);
                });
                return counts;
            }

            var ret = { required: loc.required.conjuncts.map(convert) };
            if(loc.escape) {
                ret.escape = loc.escape.conjuncts.map(convert);
            } else {
                ret.escape = [new Map()];
            }
            return ret;
        });


        // Rooms we cannot yet reach (and therefore cannot yet place an item in)
        var unreachable_locations = new Set();

        // Rooms we can reach, but haven't yet placed an item in yet.
        var reachable_unplaced_locations = [];

        // Rooms we have place an item in (and which item)
        var placed_locations = new Map();

        // A map from room_num => item_type
        var obtained_items = new Map();

        // Initialize unreachable_locations to the rooms that can be reached with
        // max_items. Rooms that cannot be reach with every item placed will be given
        // a nothing.
        for(var i = 0; i < 100; i++) {
            if(requirements_satisfied(max_items, location_reqs[i].required) &&
            requirements_satisfied(max_items, location_reqs[i].escape)) {
            unreachable_locations.add(i);
            } else {
            placed_locations.set(i, DifficultyShared.NOTHING);
            obtained_items.set(DifficultyShared.NOTHING,
                                (obtained_items.get(DifficultyShared.NOTHING) || 0) + 1);
            }
        }

        // Loop until every room can be reached
        while(true) {

            // Search for new reachable unplaced locations
            // XXX Is there a better way to iterate over a set while having the option
            //     to remove a member mid-iteration?
            [...unreachable_locations].forEach(function(room_num) {
                var room = location_reqs[room_num];
                if(requirements_satisfied(obtained_items, room.escape) &&
                requirements_satisfied(obtained_items, room.required)) {
                    reachable_unplaced_locations.push(room_num);
                    unreachable_locations.delete(room_num);
                }
            });

            if(unreachable_locations.size == 0) {
                break;
            }

            // Using the set of reachable locations, find the set of the smallest sets of pickups
            // needed to reach 1 or more additional rooms.
            var smallest_needed_count = 99;
            var smallest_needed_items = new Map();

            unreachable_locations.forEach(function(room_num) {
                var room = location_reqs[room_num];

                room.required.forEach(function(room_reqs) {
                    var [needed_item_count, needed_items] =
                            count_missing_requirements(obtained_items, room_reqs);

                    // Nested loop creates a cartesian product of the reachability
                    // requirements with the escape requirements.
                    room.escape.forEach(function(escape_reqs) {
                        var [inner_needed_item_count, inner_needed_items] =
                                count_missing_requirements(obtained_items, escape_reqs);
                        inner_needed_item_count += needed_item_count;
                        needed_items.forEach(function(item_type) {
                            inner_needed_items.add(item_type);
                        });

                        if(inner_needed_item_count == smallest_needed_count) {
                            // Merge this set of requirements into the global set
                            inner_needed_items.forEach(function(quantity, item_type) {
                                smallest_needed_items.add(item_type);
                            });
                        } else if(inner_needed_item_count < smallest_needed_count) {
                            // Replace the previous global set with this one
                            smallest_needed_count = inner_needed_item_count;
                            smallest_needed_items = inner_needed_items;
                        }
                    });
                });
            });

            if(smallest_needed_count > reachable_unplaced_locations.length) {
                return 'Sanity check failed, not enough rooms to make any other room reachable';
            }

            // Randomly select one pickup from one of the sets of pickups. The set we removed it
            // from will now be (one of) the smallest set(s) and will (potentially) have another
            // one of its members be placed next iteration.

            // XXX Is there a better way to do this than duplicating the set's contents?
            var item_list = [...smallest_needed_items];
            var random_item = item_list[Math.floor(Math.random() * item_list.length)];

            var new_obtained_items = new Map(obtained_items);
            if(!new_obtained_items.has(random_item)) {
                new_obtained_items.set(random_item, 1);
            } else {
                new_obtained_items.set(random_item, new_obtained_items.get(random_item) + 1);
            }

            // Move every room that needs random_item to have its escape condition satisfied
            // into the reachable list. They are valid candidates for placing this item and
            // will be reachable after this item is placed.
            [...unreachable_locations].forEach(function(room_num) {
                var room = location_reqs[room_num];
                // Note, using the updated obtained_items for the escape check.
                if(requirements_satisfied(new_obtained_items, room.escape) &&
                requirements_satisfied(obtained_items, room.required)) {
                    reachable_unplaced_locations.push(room_num);
                    unreachable_locations.delete(room_num);
                }
            });

            // Randomly select a room to place the item in.
            var random_room_i = Math.floor(Math.random() * reachable_unplaced_locations.length);
            var [random_room] = reachable_unplaced_locations.splice(random_room_i, 1);

            console.log("Placed " + random_item + " at " + random_room);

            placed_locations.set(random_room, random_item);
            obtained_items = new_obtained_items;
        }

        if(reachable_unplaced_locations.length + placed_locations.size != 100) {
            return 'Sanity check failed';
        }

        // Every room is reachable, but we haven't placed an item in all of them. So, do so
        // randomly.
        max_items.forEach(function(quantity, item_type) {
            var current_quantity = obtained_items.get(item_type) || 0;
            if(current_quantity > quantity) {
                return 'Sanity check failed';
            } else if(current_quantity == quantity) {
                // We've placed all of these
                return;
            }

            for(var i = current_quantity; i < quantity; i++) {
                var random_room_i = Math.floor(Math.random() * reachable_unplaced_locations.length);
                var [random_room] = reachable_unplaced_locations.splice(random_room_i, 1);

                placed_locations.set(random_room, item_type);
            }
        });
        if(reachable_unplaced_locations.length != 0) {
            return 'Sanity check failed';
        }
        console.log(placed_locations);
        var layout = [];
        for(var i = 0; i < 100; i++) {
            layout[i] = placed_locations.get(i);
        }

        return layout;
    }

    function update_nothings_count()
    {
        var sum = 0;
        $('#item_count_table input').each(function(i, input) {
            sum += parseInt($(input).val());
        });
        $('#nothings_count').text(100 - sum)
        return 100 - sum;
    }

    function display_message(msg, is_error)
    {
        var box = $('#randomize_feedback');
        if(is_error) {
            box.removeClass('alert-success');
            box.addClass('alert-danger');
        } else {
            box.removeClass('alert-danger');
            box.addClass('alert-success');
        }
        box.text(msg);
        box.css('opacity', '1');
    }

    function all_items_quantities()
    {
        return new Map([
            [DifficultyShared.MISSILE, 50],
            [DifficultyShared.ENERGY_TANK, 14],

            [DifficultyShared.THERMAL_VISOR, 1],
            [DifficultyShared.XRAY_VISOR, 1],

            [DifficultyShared.VARIA_SUIT, 1],
            [DifficultyShared.GRAVITY_SUIT, 1],
            [DifficultyShared.PHAZON_SUIT, 1],

            [DifficultyShared.MORPH_BALL, 1],
            [DifficultyShared.BOOST_BALL, 1],
            [DifficultyShared.SPIDER_BALL, 1],

            [DifficultyShared.MORPH_BALL_BOMB, 1],
            [DifficultyShared.POWER_BOMB_EXPANSION, 4],
            [DifficultyShared.POWER_BOMB, 1],

            [DifficultyShared.CHARGE_BEAM, 1],
            [DifficultyShared.SPACE_JUMP_BOOTS, 1],
            [DifficultyShared.GRAPPLE_BEAM, 1],

            [DifficultyShared.SUPER_MISSILE, 1],
            [DifficultyShared.WAVEBUSTER, 1],
            [DifficultyShared.ICE_SPREADER, 1],
            [DifficultyShared.FLAMETHROWER, 1],

            [DifficultyShared.WAVE_BEAM, 1],
            [DifficultyShared.ICE_BEAM, 1],
            [DifficultyShared.PLASMA_BEAM, 1],

            [DifficultyShared.ARTIFACT_LIFEGIVER, 1],
            [DifficultyShared.ARTIFACT_WILD, 1],
            [DifficultyShared.ARTIFACT_WORLD, 1],
            [DifficultyShared.ARTIFACT_SUN, 1],
            [DifficultyShared.ARTIFACT_ELDER, 1],
            [DifficultyShared.ARTIFACT_SPIRIT, 1],
            [DifficultyShared.ARTIFACT_TRUTH, 1],
            [DifficultyShared.ARTIFACT_CHOZO, 1],
            [DifficultyShared.ARTIFACT_WARRIOR, 1],
            [DifficultyShared.ARTIFACT_NEWBORN, 1],
            [DifficultyShared.ARTIFACT_NATURE, 1],
            [DifficultyShared.ARTIFACT_STRENGTH, 1],

            [DifficultyShared.NOTHING, 0],
        ]);
    }


    $('#randomize_button').click(function() {
        var nothings_count = update_nothings_count();
        if(nothings_count < 0) {
            display_message("There negative number of nothings is not allowed", true);
            return;
        }

        var difficulty;
        if($('input[name="difficulty"]:checked').val() == "normal") {
            difficulty = NormalDifficulty;
        } else {
            display_message("Illegal difficulty", true);
            return;
        }

        // TODO: Hard coded quantities...
        var q_name = $('input[name="quantities"]:checked').val();
        var quantities = all_items_quantities();
        if(q_name == 'all') {
            // Don't need to do anything
        } else if(q_name == 'none') {
            let optional = difficulty.optional_items;
            for(var [item_type, count] of optional) {
                quantities.set(item_type, quantities.get(item_type) - count);
                quantities.set(DifficultyShared.NOTHING, quantities.get(DifficultyShared.NOTHING) + count);
            }
        } else if(q_name == 'some') {
            let optional = difficulty.optional_items
            var optional_readd_list = [];
            var total_kept = 0;
            for(var [item_type, count] of optional) {
                // Remove 2/3rds (rounding up) of each type of item
                var count_to_keep = Math.floor(count / 3.0);
                var count_to_remove = count - count_to_keep;
                total_kept += count_to_keep;

                quantities.set(item_type, quantities.get(item_type) - count_to_remove);
                quantities.set(DifficultyShared.NOTHING,
                               quantities.get(DifficultyShared.NOTHING) + count_to_remove);

                for(var i = 0; i < count_to_remove; i++) {
                    optional_readd_list.push(item_type);
                }
            }

            // Add back items at random! Approximately enough for the amount
            // removed and the amount kept to be equal
            var count_to_add_back = (optional_readd_list.length - total_kept) / 2.0;
            // Note, we're introducing a slight variance (+/- 15%)
            count_to_add_back = Math.round(count_to_add_back * (.85 + .3 * Math.random()));

            for(var i = 0; i < count_to_add_back; i++) {
                var item_i = Math.floor(optional_readd_list.length * Math.random());
                var [item_type] = optional_readd_list.splice(item_i, 1);
                quantities.set(item_type, quantities.get(item_type) + 1);
            }
            quantities.set(DifficultyShared.NOTHING,
                           quantities.get(DifficultyShared.NOTHING) - count_to_add_back);

            console.log("Nothings: ", quantities.get(DifficultyShared.NOTHING));
        } else if(q_name == 'custom') {
            quantities = new Map([
                [DifficultyShared.MISSILE, parseInt($('#missile_count').val())],
                [DifficultyShared.ENERGY_TANK, parseInt($('#energy_tank_count').val())],

                [DifficultyShared.THERMAL_VISOR, parseInt($('#thermal_visor_count').val())],
                [DifficultyShared.XRAY_VISOR, parseInt($('#xray_visor_count').val())],

                [DifficultyShared.VARIA_SUIT, parseInt($('#varia_suit_count').val())],
                [DifficultyShared.GRAVITY_SUIT, parseInt($('#gravity_suit_count').val())],
                [DifficultyShared.PHAZON_SUIT, parseInt($('#phazon_suit_count').val())],

                [DifficultyShared.MORPH_BALL, parseInt($('#morph_ball_count').val())],
                [DifficultyShared.BOOST_BALL, parseInt($('#boost_ball_count').val())],
                [DifficultyShared.SPIDER_BALL, parseInt($('#spider_ball_count').val())],

                [DifficultyShared.MORPH_BALL_BOMB, parseInt($('#morph_ball_bomb_count').val())],
                [DifficultyShared.POWER_BOMB_EXPANSION, parseInt($('#power_bomb_expansion_count').val())],
                [DifficultyShared.POWER_BOMB, parseInt($('#power_bomb_count').val())],

                [DifficultyShared.CHARGE_BEAM, parseInt($('#charge_beam_count').val())],
                [DifficultyShared.SPACE_JUMP_BOOTS, parseInt($('#space_jump_boots_count').val())],
                [DifficultyShared.GRAPPLE_BEAM, parseInt($('#grapple_beam_count').val())],

                [DifficultyShared.SUPER_MISSILE, parseInt($('#super_missile_count').val())],
                [DifficultyShared.WAVEBUSTER, parseInt($('#wavebuster_count').val())],
                [DifficultyShared.ICE_SPREADER, parseInt($('#ice_spreader_count').val())],
                [DifficultyShared.FLAMETHROWER, parseInt($('#flamethrower_count').val())],

                [DifficultyShared.WAVE_BEAM, parseInt($('#wave_beam_count').val())],
                [DifficultyShared.ICE_BEAM, parseInt($('#ice_beam_count').val())],
                [DifficultyShared.PLASMA_BEAM, parseInt($('#plasma_beam_count').val())],

                [DifficultyShared.ARTIFACT_LIFEGIVER, parseInt($('#artifact_lifegiver_count').val())],
                [DifficultyShared.ARTIFACT_WILD, parseInt($('#artifact_wild_count').val())],
                [DifficultyShared.ARTIFACT_WORLD, parseInt($('#artifact_world_count').val())],
                [DifficultyShared.ARTIFACT_SUN, parseInt($('#artifact_sun_count').val())],
                [DifficultyShared.ARTIFACT_ELDER, parseInt($('#artifact_elder_count').val())],
                [DifficultyShared.ARTIFACT_SPIRIT, parseInt($('#artifact_spirit_count').val())],
                [DifficultyShared.ARTIFACT_TRUTH, parseInt($('#artifact_truth_count').val())],
                [DifficultyShared.ARTIFACT_CHOZO, parseInt($('#artifact_chozo_count').val())],
                [DifficultyShared.ARTIFACT_WARRIOR, parseInt($('#artifact_warrior_count').val())],
                [DifficultyShared.ARTIFACT_NEWBORN, parseInt($('#artifact_newborn_count').val())],
                [DifficultyShared.ARTIFACT_NATURE, parseInt($('#artifact_nature_count').val())],
                [DifficultyShared.ARTIFACT_STRENGTH, parseInt($('#artifact_strength_count').val())],

                [DifficultyShared.NOTHING, parseInt($('#nothings_count').text())],
            ]);
        }
        console.log(quantities);

        var layout = generate_layout(difficulty.room_reqs, quantities);

        if(typeof layout === "string") {
            display_message(layout, true);
            return;
        }

        display_message("Layout: " + LayoutString.encode_layout(layout), false);
    });

    $('#item_count_table input').change(update_nothings_count);
    $('#item_count_table input').on('input', update_nothings_count);
    update_nothings_count();

    var item_types_counts = [
        ["#missile_count", 1, 50],
        ["#energy_tank_count", 0, 14],
        ["#thermal_visor_count", 1, 5],
        ["#xray_visor_count", 1, 5],
        ["#varia_suit_count", 0, 5],
        ["#gravity_suit_count", 0, 5],
        ["#phazon_suit_count", 0, 5],
        ["#morph_ball_count", 1, 5],
        ["#boost_ball_count", 1, 5],
        ["#spider_ball_count", 0, 5],
        ["#morph_ball_bomb_count", 1, 5],
        ["#power_bomb_expansion_count", 1, 5],
        ["#power_bomb_count", 0, 5],
        ["#charge_beam_count", 0, 5],
        ["#space_jump_boots_count", 1, 5],
        ["#grapple_beam_count", 0, 5],
        ["#super_missile_count", 0, 5],
        ["#wavebuster_count", 0, 5],
        ["#ice_spreader_count", 0, 5],
        ["#flamethrower_count", 0, 5],
        ["#ice_beam_count", 1, 5],
        ["#wave_beam_count", 1, 5],
        ["#plasma_beam_count", 1, 5],
        ["#artifact_lifegiver_count", 1, 5],
        ["#artifact_wild_count", 1, 5],
        ["#artifact_world_count", 1, 5],
        ["#artifact_sun_count", 1, 5],
        ["#artifact_elder_count", 1, 5],
        ["#artifact_spirit_count", 1, 5],
        ["#artifact_truth_count", 1, 5],
        ["#artifact_chozo_count", 1, 5],
        ["#artifact_warrior_count", 1, 5],
        ["#artifact_newborn_count", 1, 5],
        ["#artifact_nature_count", 1, 5],
        ["#artifact_strength_count", 1, 5],
    ];
    item_types_counts.forEach(function(i) {
        $(i[0]).TouchSpin({
            verticalbuttons: true,
            min: i[1],
            max: i[2],
        });
    });

    // Remove spin-buttons from the tab order
    $('#item_count_table button').attr('tabindex', -1);

    // Workaround for Firefox: Make sure the table is visible after a reload if
    // the "Custom" radio is selected.
    if($('#custom_quantities').is(':checked')) {
        $('#item_count_table').collapse('show')
    }
});
