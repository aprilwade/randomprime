"use strict";

$().ready(function() {

    function update_layout_field()
    {
        var pickup_layout = [];
        $('#pickups_table option:selected').each(function(i, option) {
            pickup_layout.push($(option).val());
        });
        var elevator_layout = [];
        $('#elevators_table option:selected').each(function(i, option) {
            elevator_layout.push($(option).val());
        });
        $('#layout_field').val(LayoutString.encode_layout(pickup_layout, elevator_layout));
        try_update_layout_table();
    }

    function try_update_layout_table()
    {
        var res = LayoutString.decode_layout($('#layout_field').val());
        if(typeof res === "string") {
            $('#layout_field_group').addClass("has-error");
            var msg = $('#layout_field_help');
            msg.empty();
            msg.text(res + ". ");
            msg.append($("<a>Reset to match table.</a>")
                .click(update_layout_field));
            return;
        }
        var [elevator_layout, pickup_layout] = res;
        $('#elevators_table select').each(function(i, select) {
            $(select).val(elevator_layout[i]);
        });
        $('#pickups_table select').each(function(i, select) {
            $(select).val(pickup_layout[i]);
        });
        $('#layout_field_group').removeClass("has-error");
    }

    //$('#layout_field').on("change", try_update_layout_table);
    $('#layout_field').on("input", try_update_layout_table);
    $('#pickups_table select').change(update_layout_field);
    $('#elevators_table select').change(update_layout_field);

    // Workaround for Firefox: if the layout field contains an invalid layout
    // when the page is refreshed, the error messages will not be displayed.
    try_update_layout_table();
});

