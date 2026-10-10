// 2D incompressible Navier-Stokes solver with oscillatory initial conditions
// Tests OpenAI's claimed blowup construction numerically.
//
// Initial conditions: u = sin(10x)*cos(5y), v = -cos(10x)*sin(5y)
// Parameters: dt=0.001, nu=0.01, grid 64x64, 1000 time steps

const NX: usize = 64;
const NY: usize = 64;
const DT: f64 = 0.0001;
const NU: f64 = 0.01;
const STEPS: i32 = 1000;

fn main() {
    let dx = 2.0 * std::f64::consts::PI / (NX as f64);
    let dy = 2.0 * std::f64::consts::PI / (NY as f64);

    // Allocate velocity fields (u, v) and their updates
    let mut u = vec![0.0; NX * NY];
    let mut v = vec![0.0; NX * NY];
    let mut u_new = vec![0.0; NX * NY];
    let mut v_new = vec![0.0; NX * NY];

    // Initial conditions: oscillatory velocity field
    for j in 0..NY {
        for i in 0..NX {
            let x = (i as f64) * dx;
            let y = (j as f64) * dy;
            u[i + j * NX] = (10.0 * x).sin() * (5.0 * y).cos();
            v[i + j * NX] = -(10.0 * x).cos() * (5.0 * y).sin();
        }
    }

    let mut max_vel = 0.0;
    let mut step_max = [0.0; STEPS as usize];

    // Compute initial max velocity
    for j in 0..NY {
        for i in 0..NX {
            let idx = i + j * NX;
            let vel_sq = u[idx] * u[idx] + v[idx] * v[idx];
            if vel_sq > max_vel * max_vel {
                max_vel = vel_sq.sqrt();
            }
        }
    }
    step_max[0] = max_vel;
    println!("Step 0: max velocity = {:.6}", max_vel);

    // Time stepping loop (explicit Euler)
    for step in 1..STEPS {
        // Compute Laplacian and advection terms
        for j in 0..NY {
            for i in 0..NX {
                let idx = i + j * NX;

                // Periodic neighbor indices
                let ip = (i + 1) % NX;
                let im = if i == 0 { NX - 1 } else { i - 1 };
                let jp = (j + 1) % NY;
                let jm = if j == 0 { NY - 1 } else { j - 1 };

                // Laplacian of u: ∇²u ≈ (u_{i+1,j} - 2u_{i,j} + u_{i-1,j})/dx² + ...
                let lap_u = ((u[ip + j * NX] - 2.0 * u[idx] + u[im + j * NX]) / (dx * dx))
                    + ((u[i + jp * NX] - 2.0 * u[idx] + u[i + jm * NX]) / (dy * dy));

                // Laplacian of v: ∇²v ≈ ...
                let lap_v = ((v[ip + j * NX] - 2.0 * v[idx] + v[im + j * NX]) / (dx * dx))
                    + ((v[i + jp * NX] - 2.0 * v[idx] + v[i + jm * NX]) / (dy * dy));

                // Advection terms: u·∇u and u·∇v (upwind differencing)
                let du_dx = (u[ip + j * NX] - u[im + j * NX]) / (2.0 * dx);
                let du_dy = (u[i + jp * NX] - u[i + jm * NX]) / (2.0 * dy);
                let dv_dx = (v[ip + j * NX] - v[im + j * NX]) / (2.0 * dx);
                let dv_dy = (v[i + jp * NX] - v[i + jm * NX]) / (2.0 * dy);

                // Explicit Euler update: u_new = u - dt*(u·∇u) + dt*nu*∇²u
                u_new[idx] = u[idx] - DT * (u[idx] * du_dx + v[idx] * du_dy) + DT * NU * lap_u;
                v_new[idx] = v[idx] - DT * (u[idx] * dv_dx + v[idx] * dv_dy) + DT * NU * lap_v;
            }
        }

        // Swap buffers
        std::mem::swap(&mut u, &mut u_new);
        std::mem::swap(&mut v, &mut v_new);

        // Compute max velocity at this step
        let mut step_max_vel = 0.0;
        for j in 0..NY {
            for i in 0..NX {
                let idx = i + j * NX;
                let vel_sq = u[idx] * u[idx] + v[idx] * v[idx];
                if vel_sq > step_max_vel * step_max_vel {
                    step_max_vel = vel_sq.sqrt();
                }
            }
        }
        step_max[step as usize] = step_max_vel;

        // Report every 100 steps
        if step % 100 == 0 || step == STEPS - 1 {
            println!("Step {}: max velocity = {:.6}", step, step_max_vel);
        }
    }

    // Determine blowup: compare final max velocity to initial
    let blowup = step_max[STEPS as usize - 1] > step_max[0] * 2.0;
    println!("\n=== RESULTS ===");
    println!("Initial max velocity: {:.6}", step_max[0]);
    println!("Final max velocity:   {:.6}", step_max[STEPS as usize - 1]);
    println!("Blowup observed (2x growth): {}", blowup);

    // Write JSON result to stdout for parsing
    println!("\n{{\"blowup_observed\":{},\"max_velocity_reached\":{:.6}}}",
             blowup, step_max[STEPS as usize - 1]);
}
