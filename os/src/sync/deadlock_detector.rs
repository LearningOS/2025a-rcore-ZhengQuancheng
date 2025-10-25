use alloc::vec;
use alloc::vec::Vec;

/// Deadlock Detector structure
pub struct DeadlockDetector {
    n_threads: usize,
    n_resources: usize,
    /// vector of available resources
    avaliable_vector: Vec<isize>,
    /// matrix of needed resources
    need_matrix: Vec<Vec<isize>>,
    /// matrix of allocated resources
    allocation_matrix: Vec<Vec<isize>>,
}

impl DeadlockDetector {
    /// Create a new deadlock detector
    pub fn new() -> Self {
        Self {
            n_threads: 0,
            n_resources: 0,
            avaliable_vector: Vec::new(),
            need_matrix: Vec::new(),
            allocation_matrix: Vec::new(),
        }
    }
    /// Update state of deadlock detector
    pub fn update_state(&mut self, tid: usize, rid: usize) {
        // Update n_threads and n_resources
        self.n_threads = self.n_threads.max(tid + 1);
        self.n_resources = self.n_resources.max(rid + 1);

        // Resize avaliable_vector if needed
        if self.avaliable_vector.len() < self.n_resources {
            self.avaliable_vector.resize(self.n_resources, 0);
        }

        // Resize need_matrix if needed
        if self.need_matrix.len() < self.n_threads {
            self.need_matrix.resize(self.n_threads, vec![0; self.n_resources]);
        } else {
            for row in &mut self.need_matrix {
                if row.len() < self.n_resources {
                    row.resize(self.n_resources, 0);
                }
            }
        }

        // Resize allocation_matrix if needed
        if self.allocation_matrix.len() < self.n_threads {
            self.allocation_matrix.resize(self.n_threads, vec![0; self.n_resources]);
        } else {
            for row in &mut self.allocation_matrix {
                if row.len() < self.n_resources {
                    row.resize(self.n_resources, 0);
                }
            }
        }
        log::debug!(
            "Deadlock Detector Resized: n_threads: {}, n_resources: {}",
            self.n_threads,
            self.n_resources
        );
    }
    
    /// Update available vector
    pub fn update_available(&mut self, rid: usize, delta: isize) {
        self.update_state(0, rid);
        self.avaliable_vector[rid] += delta;
        log::debug!(
            "Deadlock Detector Available Updated: rid: {}, delta: {}, available: {:?}",
            rid,
            delta,
            self.avaliable_vector
        );
    }

    /// Update needed matrix
    pub fn update_need(&mut self, tid: usize, rid: usize, delta: isize) {
        self.update_state(tid, rid);
        self.need_matrix[tid][rid] += delta;
        log::debug!(
            "Deadlock Detector Need Updated: tid: {}, rid: {}, delta: {}, need: {:?}",
            tid,
            rid,
            delta,
            self.need_matrix
        ); 
    }

    /// Update allocation matrix
    pub fn update_allocation(&mut self, tid: usize, rid: usize, delta: isize) {
        self.update_state(tid, rid);
        self.allocation_matrix[tid][rid] += delta;
        log::debug!(
            "Deadlock Detector Allocation Updated: tid: {}, rid: {}, delta: {}, allocation: {:?}",
            tid,
            rid,
            delta,
            self.allocation_matrix
        );
    }

    /// Check safety of the system
    pub fn check_safety(&self) -> bool {
        let mut work = self.avaliable_vector.clone();
        let mut finish = vec![false; self.need_matrix.len()];
        loop {
            let mut found = false;
            for (i, need) in self.need_matrix.iter().enumerate() {
                if !finish[i] && need.iter().enumerate().all(|(j, &n)| n <= work[j]) {
                    for j in 0..work.len() {
                        work[j] += self.allocation_matrix[i][j];
                    }
                    finish[i] = true;
                    found = true;
                }
            }
            if !found {
                break;
            }
            log::debug!("Deadlock Detector State: work: {:?}, finish: {:?}", work, finish);
        }
        finish.into_iter().all(|f| f)
    }
}
