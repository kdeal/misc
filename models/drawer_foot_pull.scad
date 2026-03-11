// Drawer foot-pull hook
// Units: mm
//
// Purpose:
// Mount this on the underside/front edge of a drawer so you can catch it
// with your foot and pull the drawer open hands-free.

$fn = 48;

back_thickness = 2.5;
std_thickness = 2.5;

width = 90;
back_size = 20;
under_size = 10;
drop_size = 10;

support_width = 5;

// screw size
screw_diameter = 2.75;
screw_head = 5.5;
screw_head_depth = 2.5;

module screw_hole() {
    rotate(a=[0, 90, 0])
    union() {
        cylinder(h=back_thickness, d=screw_diameter);
        cylinder(h=screw_head_depth, d2=screw_diameter, d1=screw_head);
    }  
}

module body() {
    union() {
        cube([back_thickness, width, back_size]);
        cube([under_size, width, std_thickness]);
        translate(v=[under_size - std_thickness, 0, -drop_size])
        cube([std_thickness, width, drop_size]);
        translate(v=[under_size - std_thickness - support_width, 0, 0])
        polyhedron(
            [
                [0, 0, 0],
                [support_width, 0, 0],
                [support_width, 0, -support_width],
                [0, width, 0],
                [support_width, width, 0],
                [support_width, width, -support_width],
            ],
            [
                [0, 1, 2],
                [0, 1, 4, 3],
                [1, 2, 5, 4],
                [0, 2, 5, 3],
                [3, 4, 5],
            ]
        );
    }
}

difference() {
    body();
    translate(v=[0, 0, back_size - (screw_head/2) - 2]) {
        translate(v=[0, (screw_head/2) + 2, 0])
        screw_hole();
        translate(v=[0, (width/2), 0])
        screw_hole();
        translate(v=[0, width - (screw_head/2) - 2, 0])
        screw_hole();
    }
}
