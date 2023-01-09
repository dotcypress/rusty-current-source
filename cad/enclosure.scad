epsilon=0.01;
depth=24;
width=33;
jack_spacing=19.05;
cr=2;

module button() {
  cube([15, 20, 3], center=true);
}

module pcb(height) {
  cube([29.6, 19.6, height], center=true);
}

module battery() {
  cube([26, 18, 50], center=true);
}

module led() {
  cube([2, 5, 10], center=true);
}

module jack(color="#773333") {
  color("#444444")
    translate([0, 0, -3.5])
    cylinder(r=1.6, h=11.5, center=true);
  color(color)
    translate([0, 0, 1.25+epsilon])
    cylinder(r=4, h=2.5, center=true);
}

module body(height) {
  hull() {
    translate([width/2-cr, depth/2-cr, 0]) cylinder(r=cr, h=height, center=true);
    translate([-width/2+cr, depth/2-cr, 0]) cylinder(r=cr, h=height, center=true);
    translate([width/2-cr, -depth/2+cr, 0]) cylinder(r=cr, h=height, center=true);
    translate([-width/2+cr, -depth/2+cr, 0]) cylinder(r=cr, h=height, center=true);
  }
}

module enclosure() {
  difference() {
    body(60);
    union() {
      translate([0, -1, -3])
        battery();
      translate([0, -1, 26])
        pcb(10);
      translate([0, depth/2-2.7, 20+epsilon])
        rotate([90, 0, 0])
        button();
    }
  }
}

module hat(height=7) {
  difference() {
    union() {
      body(height);
      translate([0, -1, -2]) pcb(height);
    }
    union() {
      translate([jack_spacing/2, -4.5, height/2]) jack();
      translate([-jack_spacing/2, -4.5, height/2]) jack();
      translate([jack_spacing/2, -4.5, -4]) cylinder(r=5, h=height, center=true);
      translate([-jack_spacing/2, -4.5, -4]) cylinder(r=5, h=height, center=true);
      for (i=[-2:2]) {
        translate([-6*i, 5, height/2-5+epsilon]) 
          rotate([0, 0, -0]) 
          led();
      }
      translate([0, -1, -4]) 
        cube([27, 17, height], center=true);
    }
  }
}

$fn=128;
enclosure();
translate([0, 0, 50]) 
  hat();
