"use strict";

var LayoutString = (function() {
    var TABLE = "ABCDEFGHIJKLMNOPQRSTUWVXYZabcdefghijklmnopqrstuwvxyz0123456789-_";
    var REV_TABLE = new Map();
    for(var i = 0; i < TABLE.length; i++) {
        REV_TABLE.set(TABLE[i], i);
    }

    function compute_checksum(layout_number)
    {
        var s = 0;
        while(layout_number.greater(0)){
            var divmod = layout_number.divmod(32);
            s = (s + divmod.remainder) % 32;
            layout_number = divmod.quotient;
        }
        return s;
    }

    function encode_pickup_layout(layout)
    {
        var num = bigInt(0);
        layout.forEach(function(item_type) {
            num = num.times(36).plus(item_type);
        });

        var checksum = compute_checksum(num);
        num = num.plus(bigInt(checksum).shiftLeft(517));

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
        for(var i = 0; i < 87; i++) {
            var divmod = num.divmod(64);
            num = divmod.quotient;

            s = s + TABLE[divmod.remainder];
        }

        return s;
    }

    function decode_pickup_layout(layout_string)
    {
        if(layout_string.length != 87) {
            return 'Invalid layout: incorrect legnth, not 87 characters';
        }
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

        var checksum_value = num.shiftRight(517);
        num = num.minus(checksum_value.shiftLeft(517));
        checksum_value = checksum_value.toJSNumber();
        if(checksum_value != compute_checksum(num)) {
            return 'Invalid layout: checksum failed';
        }

        var layout = [];
        for(var i = 0; i < 100; i++) {
            var divmod = num.divmod(36);
            layout.push(divmod.remainder.toJSNumber());
            num = divmod.quotient;
        }

        layout.reverse();
        return layout;
    }

    return {
        decode_pickup_layout: decode_pickup_layout,
        encode_pickup_layout: encode_pickup_layout,
    };
}());
