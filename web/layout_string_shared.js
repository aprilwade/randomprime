"use strict";

var LayoutString = (function() {
    var TABLE = "ABCDEFGHIJKLMNOPQRSTUWVXYZabcdefghijklmnopqrstuwvxyz0123456789-_";
    var REV_TABLE = new Map();
    for(var i = 0; i < TABLE.length; i++) {
        REV_TABLE.set(TABLE[i], i);
    }
    var PICKUP_SIZES = Array(100).fill(36);
    var PICKUP_SIZES_2 = Array(100).fill(37);
    var ELEVATOR_SIZES = Array(20).fill(20).concat([21]);

    function compute_checksum(checksum_size, layout_number)
    {
        if(checksum_size == 0) {
            return 0;
        }
        var s = 0;
        while(layout_number.greater(0)){
            var divmod = layout_number.divmod(1 << checksum_size);
            s = (s + divmod.remainder) % (1 << checksum_size);
            layout_number = divmod.quotient;
        }
        return s;
    }

    function encode_layout(pickup_layout, elevator_layout)
    {
        var elevator_string;
        if(elevator_layout === undefined) {
            elevator_string = "qzoCAr2fwehJmRjM"
        } else {
            elevator_string = encode_layout_inner(ELEVATOR_SIZES, 91, 5, elevator_layout);
        }

        var pickup_string;
        if(pickup_layout.indexOf("36") == -1) {
            pickup_string = encode_layout_inner(PICKUP_SIZES, 517, 5, pickup_layout);
        } else {
            pickup_string = "!" + encode_layout_inner(PICKUP_SIZES_2, 521, 1, pickup_layout);
        }

        if(elevator_string != "qzoCAr2fwehJmRjM") {
            return elevator_string + "." + pickup_string;
        } else {
            return pickup_string;
        }
    }

    function encode_layout_inner(sizes, layout_data_size, checksum_size, layout)
    {
        var num = bigInt(0);
        layout.forEach(function(item_type, i) {
            num = num.times(sizes[i]).plus(item_type);
        });

        var checksum = compute_checksum(checksum_size, num);
        num = num.plus(bigInt(checksum).shiftLeft(layout_data_size));

        var even_bits = [];
        var odd_bits = [];
        var all_bits = num.toString(2);
        for(var i = 0; i < all_bits.length; i++) {
            if(i % 2) {
                odd_bits.push(all_bits[i]);
            } else {
                even_bits.push(all_bits[i]);
            }
        }

        odd_bits.reverse();
        all_bits = []
        for(var i = 0; i < even_bits.length; i++) {
            all_bits.push(even_bits[i]);
            all_bits.push(odd_bits[i]);
        }
        num = bigInt(all_bits.join(""), 2)

        var s = '';
        for(var i = 0; i < layout_data_size / 6; i++) {
            var divmod = num.divmod(64);
            num = divmod.quotient;

            s = s + TABLE[divmod.remainder];
        }

        return s;
    }

    function decode_layout(layout_string)
    {
        var pickup_layout, elevator_layout;
        if(layout_string.includes('.')) {
            [elevator_layout, pickup_layout] = layout_string.split('.');
            if(elevator_layout.length != 16) {
                return "Invalid layout: incorrect length for the section before '.', not 16 characters";
            }
        } else {
            pickup_layout = layout_string;
            has_scan_visor = pickup_layout[0] == '!';
            elevator_layout = "qzoCAr2fwehJmRjM";
        }

        var has_scan_visor = false;
        if(pickup_layout[0] == '!') {
            has_scan_visor = true;
            pickup_layout = pickup_layout.substring(1);
        }

        if(pickup_layout.length != 87) {
            return "Invalid layout: incorrect length for the section after '.', not 87 characters";
        }

        var el = decode_layout_inner(ELEVATOR_SIZES, 91, 5, elevator_layout);
        if(typeof el === "string") {
            return el;
        }

        var pl;
        if(has_scan_visor) {
            pl  = decode_layout_inner(PICKUP_SIZES_2, 521, 1, pickup_layout);
        } else {
            pl  = decode_layout_inner(PICKUP_SIZES, 517, 5, pickup_layout);
        }
        if(typeof pl === "string") {
            return pl;
        }
        return [el, pl];
    }

    function decode_layout_inner(sizes, layout_data_size, checksum_size, layout_string)
    {
        var num = bigInt(0);
        for(var i = layout_string.length - 1; i >= 0; i--) {
            num = num.shiftLeft(6).plus(REV_TABLE.get(layout_string[i]));
        }

        var even_bits = [];
        var odd_bits = [];
        var all_bits = num.toString(2);
        for(var i = 0; i < all_bits.length; i++) {
            if(i % 2) {
                odd_bits.push(all_bits[i]);
            } else {
                even_bits.push(all_bits[i]);
            }
        }

        odd_bits.reverse();
        all_bits = []
        for(var i = 0; i < even_bits.length; i++) {
            all_bits.push(even_bits[i]);
            all_bits.push(odd_bits[i]);
        }
        num = bigInt(all_bits.join(""), 2)

        var checksum_value = num.shiftRight(layout_data_size);
        num = num.minus(checksum_value.shiftLeft(layout_data_size));
        checksum_value = checksum_value.toJSNumber();
        if(checksum_value != compute_checksum(checksum_size, num)) {
            return 'Invalid layout: checksum failed';
        }

        var layout = [];
        sizes = sizes.slice().reverse();
        sizes.forEach(function(denum) {
            var divmod = num.divmod(denum);
            layout.push(divmod.remainder.toJSNumber());
            num = divmod.quotient;
        });

        layout.reverse();
        return layout;
    }

    return {
        decode_layout: decode_layout,
        encode_layout: encode_layout,
    };
}());
