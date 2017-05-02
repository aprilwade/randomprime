"use strict";

$().ready(function() {

    function update_layout_field()
    {
        var layout = [];
        $('#pickups_table option:selected').each(function(i, option) {
            layout.push($(option).val());
        });
        $('#layout_field').val(LayoutString.encode_pickup_layout(layout));
        try_update_layout_table();
    }

    function try_update_layout_table()
    {
        var layout = LayoutString.decode_pickup_layout($('#layout_field').val());
        if(typeof layout === "string") {
            $('#layout_field_group').addClass("has-error");
            var msg = $('#layout_field_help');
            msg.empty();
            msg.text(layout + ". ");
            msg.append($("<a>Reset to match table.</a>")
                .click(update_layout_field));
            return;
        }
        $('#pickups_table select').each(function(i, select) {
            $(select).val(layout[i]);
        });
        $('#layout_field_group').removeClass("has-error");
    }

    //$('#layout_field').on("change", try_update_layout_table);
    $('#layout_field').on("input", try_update_layout_table);
    $('#pickups_table select').change(update_layout_field);

    // Workaround for Firefox: if the layout field contains an invalid layout
    // when the page is refreshed, the error messages will not be displayed.
    try_update_layout_table();
});

