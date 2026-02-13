// L-shaped tapered cover
// Units: mm

$fn = 64;

// Base L dimensions (top face)
thickness = 3;
leg_length_x = 95;
leg_length_y = 95;
leg_width = 60;

// Corner brace
support_x = 10;
support_y = 30;


// screw size
screw_diameter = 2.75;
screw_head = 5.5;
screw_head_depth = 2.5;
screw_from_edge = 5 + (screw_head / 2);
screw_from_support = support_x + (screw_head / 2) + 1;

// taper
taper_width = 10;
taper_length = leg_length_x - support_x - screw_from_edge - screw_head;

module screw_hole() {
  union() {
    cylinder(h=thickness, d=screw_diameter);
    cylinder(h=screw_head_depth, d2=screw_diameter, d1=screw_head);
  }  
}

module taper() {
  polyhedron(points=[
      [0, 0, 0],
      [0, thickness, taper_width],
      [0, thickness, 0],
      [taper_length, 0, 0],
      [taper_length, thickness, taper_width],
      [taper_length, thickness, 0]
    ],
    faces=[
      [0, 1, 2],
      [0, 1, 4, 3],
      [0, 2, 5, 3],
      [1, 2, 5, 4],
      [3, 4, 5]
    ]);
}

difference() {
  union() {
    // X leg
    cube([leg_length_x, thickness, leg_width]);

    // Y leg
    rotate(90)
    cube([leg_length_y, thickness, leg_width]);
    
    // Corner brace
    translate([0, thickness, 0])
    polyhedron(points=[
      [0, 0, 0],
      [support_x, 0, 0],
      [0, support_y, 0],
      [0, 0, leg_width],
      [support_x, 0, leg_width],
      [0, support_y, leg_width]
    ],
    faces=[
      [0, 1, 2],
      [0, 1, 4, 3],
      [0, 2, 5, 3],
      [1, 2, 5, 4],
      [3, 4, 5]
    ]);
  }

  // bottom y hole
  translate([0, leg_length_y - 10, screw_from_edge])
  rotate(a=[0, 90, 180])
  screw_hole();

  // top y hole
  translate([0, leg_length_y - 10, leg_width - screw_from_edge])
  rotate(a=[0, 90, 180])
  screw_hole();

  // bottom x hole
  translate([screw_from_support, thickness, screw_from_edge])
  rotate(a=[90, 0, 0])
  screw_hole();

  // top x hole
  translate([screw_from_support, thickness, leg_width - screw_from_edge])
  rotate(a=[90, 0, 0])
  screw_hole();

  // bottom taper
  translate([leg_length_x - taper_length, 0, 0])
  taper();
  // top taper
  translate([leg_length_x, 0, leg_width])
  rotate([180, 0, 180])
  taper();
}
