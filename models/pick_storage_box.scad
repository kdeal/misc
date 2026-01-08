// Guitar Pick Case – side-by-side compartments
// Left: vertical pick slots with half-height dividers
// Right: bulk storage
// Two sliding lids with rails, finger dents,
// and downward retention bumps on lids

//--------------------------------------------------
// Main parameters
//--------------------------------------------------
wall_th       = 2;
floor_th      = 2;

inner_x_comp  = 35;      // interior X of each compartment
inner_z       = 35;      // interior Z (height)

case_w        = 80;      // overall Y
inner_w       = case_w - 2*wall_th;
base_h        = floor_th + inner_z;

lid_th        = 3;

// vertical pick slots (left compartment)
pick_slot_count = 10;
pick_slot_gap   = 2.2;   // free width for a pick on edge
pick_div_th     = 1.2;   // divider thickness

// sliding rails + clearances
rail_w       = 2;        // rail width in Y
rail_h       = 1.2;      // rail height inside lid slot (<= lid_th)
lid_clear_xy = 0.3;
mouth_len    = 10;       // open front length for sliding lids

// finger dent in lids
dent_r       = 4;

// lid retention bumps (downwards)
bump_len     = 3;        // along X
bump_w_y     = 1.0;      // along Y
bump_h       = 1.0;      // downward height (negative Z)

// center wall height: below lid slot so lids pass over it
center_wall_top = base_h - lid_th - 0.4;  // 0.4 mm clearance

//--------------------------------------------------
// X layout
//--------------------------------------------------
x_left_in0      = wall_th;
x_left_in1      = x_left_in0 + inner_x_comp;

x_center_wall0  = x_left_in1;
x_center_wall1  = x_center_wall0 + wall_th;

x_right_in0     = x_center_wall1;
x_right_in1     = x_right_in0 + inner_x_comp;

case_len        = x_right_in1 + wall_th;

//--------------------------------------------------
// Outer shell (no center wall yet)
//--------------------------------------------------
module shell_without_center() {
    difference() {
        // outer box
        cube([case_len, case_w, base_h]);

        // interior cavity for both compartments
        translate([wall_th, wall_th, floor_th])
            cube([case_len - 2*wall_th,
                  inner_w,
                  inner_z]);
    }
}

//--------------------------------------------------
// Center wall (shorter so lids pass over it)
//--------------------------------------------------
module center_wall() {
    translate([x_center_wall0, wall_th, floor_th])
        cube([wall_th, inner_w, center_wall_top - floor_th]);
}

//--------------------------------------------------
// Half-height vertical pick dividers (left compartment)
//--------------------------------------------------
module pick_vertical_dividers() {
    n       = pick_slot_count;
    gap_w   = pick_slot_gap;
    div_th  = pick_div_th;
    div_h   = inner_z / 2;

    total_len = n*gap_w + (n-1)*div_th;
    margin    = (inner_x_comp - total_len) / 2;
    x_start   = x_left_in0 + margin;

    for (i = [0 : n-2]) {
        x_div = x_start + gap_w*(i+1) + div_th*i;
        translate([x_div, wall_th, floor_th])
            cube([div_th, inner_w, div_h]);
    }
}

//--------------------------------------------------
// Sliding-lid rails (no bumps on rails)
//--------------------------------------------------
module lid_rails() {
    // bottom of lid slot
    z_slot_bottom = base_h - lid_th;

    // LEFT rails
    translate([x_left_in0, wall_th, z_slot_bottom])
        cube([inner_x_comp, rail_w, rail_h]); // front rail
    translate([x_left_in0, case_w - wall_th - rail_w, z_slot_bottom])
        cube([inner_x_comp, rail_w, rail_h]); // back rail

    // RIGHT rails
    translate([x_right_in0, wall_th, z_slot_bottom])
        cube([inner_x_comp, rail_w, rail_h]);
    translate([x_right_in0, case_w - wall_th - rail_w, z_slot_bottom])
        cube([inner_x_comp, rail_w, rail_h]);
}

//--------------------------------------------------
// Mouth cut – ONLY the lid slot region at the front
//--------------------------------------------------
module mouth_cut() {
    z_slot_bottom = base_h - lid_th;
    translate([0, 0, z_slot_bottom])
        cube([mouth_len, case_w, lid_th]);  // open slot for lids to slide
}

//--------------------------------------------------
// Base Assembly
//--------------------------------------------------
module base() {
    difference() {
        union() {
            shell_without_center();
            center_wall();
            pick_vertical_dividers();
            lid_rails();
        }
        mouth_cut();
    }
}

//--------------------------------------------------
// Lids – with finger dents and downward bumps
//--------------------------------------------------
module left_lid() {
    lid_len_x = inner_x_comp + mouth_len;
    lid_len_y = inner_w - 2*rail_w - 2*lid_clear_xy;
    y0        = wall_th + rail_w + lid_clear_xy;

    // X position where bumps should end up “behind” the back outer wall
    // Back inner wall for left compartment is at x = wall_th + inner_x_comp.
    // Put bump slightly further into the box.
    x_bump    = wall_th + inner_x_comp + 0.5;

    // place on print bed at origin
    // move to the side for printing
    translate([- lid_len_x - 5, 0, 0])
    union() {
        // plate with finger dent
        difference() {
            translate([0, y0, 0])
                cube([lid_len_x, lid_len_y, lid_th]);

            // finger dent at front
            translate([-dent_r + mouth_len/2,
                       wall_th + inner_w/2 - lid_clear_xy,
                       -0.01])
                cylinder(r=dent_r, h=lid_th + 0.02, $fn=32);
        }

        // two downward bumps (front & back edges in Y)
        translate([x_bump - bump_len/2, y0, -bump_h])
            cube([bump_len, bump_w_y, bump_h]);  // near front rail

        translate([x_bump - bump_len/2, y0 + lid_len_y - bump_w_y, -bump_h])
            cube([bump_len, bump_w_y, bump_h]);  // near back rail
    }
}

module right_lid() {
    lid_len_x = inner_x_comp + mouth_len;
    lid_len_y = inner_w - 2*rail_w - 2*lid_clear_xy;
    y0        = wall_th + rail_w + lid_clear_xy;

    // For right lid: bump should click behind the center divider.
    // Center divider right face at x = x_center_wall1.
    x_bump    = x_center_wall1 + 0.5;

    // move to the side for printing
    translate([case_len + 5, 0, 0])
    union() {
        // plate with finger dent
        difference() {
            translate([0, y0, 0])
                cube([lid_len_x, lid_len_y, lid_th]);

            // finger dent
            translate([-dent_r + mouth_len/2,
                       wall_th + inner_w/2 - lid_clear_xy,
                       -0.01])
                cylinder(r=dent_r, h=lid_th + 0.02, $fn=32);
        }

        // downward bumps, front & back
        translate([x_bump - bump_len/2, y0, -bump_h])
            cube([bump_len, bump_w_y, bump_h]);

        translate([x_bump - bump_len/2, y0 + lid_len_y - bump_w_y, -bump_h])
            cube([bump_len, bump_w_y, bump_h]);
    }
}

//--------------------------------------------------
// Final Layout
//--------------------------------------------------
base();
left_lid();
right_lid();
