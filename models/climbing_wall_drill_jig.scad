//
// Climbing-wall drilling jig
// – Semicircle ends
// – Locator holes on BOTH sides
// – Drill block: top width matches jig width (40 mm)
// – Pins restored to original length (5 mm)
// – Clean rounded edges, tongue into slot
// Units: mm
//

inch = 25.4;

// ---------------- Parameters ----------------

// Bolt spacing
bolt_spacing_inch   = 6;
bolt_spacing        = bolt_spacing_inch * inch;

// Pegs for jig
peg_diameter        = 7.3;
peg_length          = 15.0;

// Jig body geometry
body_thickness      = 10.0;
body_width          = 40.0;       // <<< full jig width
end_margin          = 15.0;

// Slot
slot_width          = 18.0;
slot_end_margin     = 10.0;
slot_length         = bolt_spacing - 2*slot_end_margin;
slot_clearance      = 0.3;

// Drill guide hole
drill_bit_diameter_inch = 9/64;
drill_bit_diameter      = drill_bit_diameter_inch * inch;
guide_clearance         = 0.6;
guide_radius            = (drill_bit_diameter + guide_clearance) / 2;

// Indexing (1/8”)
step_inch        = 1/8;
step_mm          = step_inch * inch;
max_steps        = floor((slot_length/2) / step_mm);

// Locator holes + pins
locator_diameter       = 6.0;
locator_clearance      = 0.3;
locator_hole_depth     = 6.0;
locator_pin_length     = 5.0;   // <<< restored original length
locator_edge_margin_body = 6.0;

// Drill block
block_length        = 24.0;
block_width         = body_width;   // <<< drill block matches jig width
block_height_above  = 6.0;
fillet_r_block      = 2.0;

// Tongue (still narrower)
tongue_height      = 8.0;
tongue_length      = block_length - 4.0;
tongue_width       = slot_width - 2*slot_clearance;

// ---------------- Derived ----------------

peg_radius   = peg_diameter / 2;
body_length  = bolt_spacing + 2*end_margin;
end_radius   = body_width / 2;

locator_row_y = body_width/2 - locator_edge_margin_body;
pin_y_block   = locator_row_y;

// ---------------- Helpers ----------------

// Capsule-shaped jig profile
module body_profile_2d() {
    dist_centers = body_length - 2*end_radius;
    xoff = dist_centers / 2;

    hull() {
        translate([-xoff, 0]) circle(r=end_radius, $fn=64);
        translate([ xoff, 0]) circle(r=end_radius, $fn=64);
    }
}

// Rounded rectangle for drill block
module rounded_rect_2d(L, W, r) {
    offset(r = r)
        square([L - 2*r, W - 2*r], center = true);
}

// ---------------- Jig body ----------------

module jig_body() {
    difference() {
        linear_extrude(height = body_thickness)
            body_profile_2d();

        translate([ -slot_length/2, -slot_width/2, -0.1 ])
            cube([ slot_length, slot_width, body_thickness+0.2 ]);

        for (k = [-max_steps : max_steps]) {
            x = k * step_mm;

            translate([ x,  locator_row_y, body_thickness - locator_hole_depth ])
                cylinder(h=locator_hole_depth+0.2,
                         r=(locator_diameter+locator_clearance)/2, $fn=32);

            translate([ x, -locator_row_y, body_thickness - locator_hole_depth ])
                cylinder(h=locator_hole_depth+0.2,
                         r=(locator_diameter+locator_clearance)/2, $fn=32);
        }
    }

    translate([ -bolt_spacing/2, 0, -peg_length ])
        cylinder(h=peg_length, r=peg_radius, $fn=32);

    translate([  bolt_spacing/2, 0, -peg_length ])
        cylinder(h=peg_length, r=peg_radius, $fn=32);
}

// ---------------- Drill block ----------------

module drill_block() {
    difference() {
        union() {
            // Main rounded block (full jig width)
            linear_extrude(height = block_height_above)
                rounded_rect_2d(block_length, block_width, fillet_r_block);

            // Tongue
            translate([ -tongue_length/2, -tongue_width/2, -tongue_height ])
                cube([ tongue_length, tongue_width, tongue_height ]);

            // Pins both sides (5 mm deep)
            translate([ 0,  pin_y_block, -locator_pin_length ])
                cylinder(h=locator_pin_length,
                         r=locator_diameter/2, $fn=32);

            translate([ 0, -pin_y_block, -locator_pin_length ])
                cylinder(h=locator_pin_length,
                         r=locator_diameter/2, $fn=32);
        }

        // Drill-through hole
        translate([ 0, 0, -100 ])
            cylinder(h=200, r=guide_radius, $fn=64);
    }
}

// ---------------- View ----------------

jig_body();

// translate([0, block_width + 5, 0])
// drill_block();
