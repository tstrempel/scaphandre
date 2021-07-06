use crate::current_system_time_since_epoch;
use crate::exporters::*;
use crate::sensors::{Sensor, Topology};
use clap::Arg;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

extern crate systemstat;
use systemstat::{System, Platform, CPULoad};
use std::sync::mpsc;

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct JSONExporter {
    topology: Topology,
    reports: Vec<Report>,
}

impl Exporter for JSONExporter {
    /// Lanches runner()
    fn run(&mut self, parameters: ArgMatches) {
        self.runner(parameters);
    }

    /// Returns options needed for that exporter, as a HashMap

    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        let mut options = Vec::new();
        let arg = Arg::with_name("timeout")
            .default_value("10")
            .help("Maximum time spent measuring, in seconds.")
            .long("timeout")
            .short("t")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("step_duration")
            .default_value("2")
            .help("Set measurement step duration in second.")
            .long("step")
            .short("s")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("step_duration_nano")
            .default_value("0")
            .help("Set measurement step duration in nano second.")
            .long("step_nano")
            .short("n")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("file_path")
            .default_value("")
            .help("Destination file for the report.")
            .long("file")
            .short("f")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("max_top_consumers")
            .default_value("10")
            .help("Maximum number of processes to watch.")
            .long("max-top-consumers")
            .short("m")
            .required(false)
            .takes_value(true);
        options.push(arg);

        options
    }
}

#[derive(Serialize, Deserialize)]
struct Domain {
    name: String,
    consumption: f32,
}
#[derive(Serialize, Deserialize)]
struct Socket {
    id: u16,
    consumption: f32,
    domains: Vec<Domain>,
}

#[derive(Serialize, Deserialize)]
struct Consumer {
    exe: PathBuf,
    pid: i32,
    consumption: f32,
}
#[derive(Serialize, Deserialize)]
struct Host {
    consumption: f32,
    timestamp: f64,
    average_load: f32,
    cpu_load: f32,
}
#[derive(Serialize, Deserialize)]
struct Report {
    host: Host,
    consumers: Vec<Consumer>,
    sockets: Vec<Socket>,
}

impl JSONExporter {
    /// Instantiates and returns a new JSONExporter
    pub fn new(mut sensor: Box<dyn Sensor>) -> JSONExporter {
        let some_topology = *sensor.get_topology();
        
        JSONExporter {
            topology: some_topology.unwrap(),
            reports: Vec::new(),
        }
    }

    /// Runs iteration() every 'step', until 'timeout'
    pub fn runner(&mut self, parameters: ArgMatches) {
        // transmit, recieve
        let (sender, receiver) = mpsc::channel();
    

        let timeout = parameters.value_of("timeout").unwrap();
        if timeout.is_empty() {
            thread::spawn(move || {       
                let sys = System::new();
                    
                loop {
                    match sys.cpu_load_aggregate() {
                        Ok(cpu)=> {
                            thread::sleep(Duration::new(2, 0));
                            let cpu = cpu.done().unwrap();
                            sender.send(cpu);
                        },
                        Err(x) => println!("\nCPU load: error: {}", x)
                    }
                }
            });
            thread::sleep(Duration::from_secs(2));

            self.iterate(&parameters, &receiver);
        } else {
            let now = Instant::now();

            let timeout_secs: u64 = timeout.parse().unwrap();

            // We have a default value of 2s so it is safe to unwrap the option
            // Panic if a non numerical value is passed
            let step_duration: u64 = parameters
                .value_of("step_duration")
                .unwrap()
                .parse()
                .expect("Wrong step_duration value, should be a number of seconds");
            let step_duration_nano: u32 = parameters
                .value_of("step_duration_nano")
                .unwrap()
                .parse()
                .expect("Wrong step_duration_nano value, should be a number of nano seconds");

            info!("Measurement step is: {}s", step_duration);

            thread::spawn(move || {  
                let sys = System::new();
                    
                loop {
                    match sys.cpu_load_aggregate() {
                        Ok(cpu)=> {
                            thread::sleep(Duration::new(step_duration, step_duration_nano));
                            let cpu = cpu.done().unwrap();
                            sender.send(cpu);
                        },
                        Err(x) => println!("\nCPU load: error: {}", x)
                    }
                }
            });
            thread::sleep(Duration::new(step_duration, step_duration_nano));

            while now.elapsed().as_secs() <= timeout_secs {
                self.iterate(&parameters, &receiver);
                thread::sleep(Duration::new(step_duration, step_duration_nano));
            }
        }
        drop(receiver);
    }

    fn iterate(&mut self, parameters: &ArgMatches, receiver: &mpsc::Receiver<CPULoad>) {
        self.topology.refresh();
        self.retrieve_metrics(&parameters, &receiver);
    }

    fn retrieve_metrics(&mut self, parameters: &ArgMatches, receiver: &mpsc::Receiver<CPULoad>) {
        let mut host_power = 0;
        let host_timestamp: Duration;
        if let Some(microwatts_record) = self.topology.get_records_diff_power_microwatts() {
            host_timestamp = microwatts_record.timestamp;

            host_power = microwatts_record.value.parse::<u64>().unwrap();
        } else {
            host_timestamp = current_system_time_since_epoch();
        }
        
        let sys = System::new();
        let mut host_average_load = 0.0;
        /*match sys.load_average() {
            Ok(loadavg) => {
                host_average_load = loadavg.one;
                println!("{}", host_average_load);
            },
            Err(x) => println!("\nLoad average: error: {}", x)
        }*/
        host_average_load = sys.load_average().unwrap().one;
       
        let tmp_cpu_load = receiver.try_recv().unwrap();
        let cpu_load = receiver.try_recv().unwrap_or(tmp_cpu_load);
        let host_cpu_load = cpu_load.user + cpu_load.nice + cpu_load.system;

        /*let mpstat = Command::new("/usr/bin/mpstat")
                     .output()
                     .expect("failed to execute process");
        let mpstat_string = String::from_utf8_lossy(&mpstat.stdout);
        let non_idle = 1.0 - (mpstat_string
            .lines().last().unwrap()
            .split_whitespace().last().unwrap()
            .parse::<f32>().unwrap() / 100.0);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));*/


        let host_stat = match self.topology.get_stats_diff() {
            Some(value) => value,
            None => return,
        };
        // let host_cpu_load = host_stat.cputime.user + host_stat.cputime.nice + host_stat.cputime.system;

        let consumers = self.topology.proc_tracker.get_top_consumers(
            parameters
                .value_of("max_top_consumers")
                .unwrap_or("10")
                .parse::<u16>()
                .unwrap(),
        );
        let top_consumers = consumers
            .iter()
            .map(|(process, value)| {
                let host_time = host_stat.total_time_jiffies();
                Consumer {
                    exe: process.exe().unwrap_or_default(),
                    pid: process.pid,
                    consumption: ((*value as f32
                        / (host_time * procfs::ticks_per_second().unwrap() as f32))
                        * host_power as f32),
                }
            })
            .collect::<Vec<_>>();

        // let names = ["core", "uncore", "dram"];
        let all_sockets = self
            .topology
            .get_sockets_passive()
            .iter()
            .map(|socket| {
                let socket_power = socket
                    .get_records_diff_power_microwatts()
                    .map(|record| record.value.parse::<u64>().unwrap())
                    .unwrap_or(0);

                let domains = socket
                    .get_domains_passive()
                    .iter()
                    .map(|d| (d.name.clone(), d.get_records_diff_power_microwatts()))
                    .map(|(n, record)| (n, record.map(|d| d.value)))
                    .map(|(name, d)| {
                        let domain_power =
                            d.map(|value| value.parse::<u64>().unwrap()).unwrap_or(0);
                        Domain {
                            name,
                            consumption: domain_power as f32,
                        }
                    })
                    .collect::<Vec<_>>();

                Socket {
                    id: socket.id,
                    consumption: (socket_power as f32),
                    domains,
                }
            })
            .collect::<Vec<_>>();

        let host_report = Host {
            consumption: host_power as f32,
            timestamp: host_timestamp.as_secs_f64(),
            average_load: host_average_load as f32,
            cpu_load: host_cpu_load as f32,
        };

        let report = Report {
            host: host_report,
            consumers: top_consumers,
            sockets: all_sockets,
        };

        let file_path = parameters.value_of("file_path").unwrap();
        // Print json
        if file_path.is_empty() {
            let json: String = serde_json::to_string(&report).expect("Unable to parse report");
            println!("{}", &json);
        } else {
            self.reports.push(report);
            // Serialize it to a JSON string.
            let json: String =
                serde_json::to_string(&self.reports).expect("Unable to parse report");
            let _ = File::create(file_path);
            fs::write(file_path, &json).expect("Unable to write file");
        }
    }
}

#[cfg(test)]
mod tests {
    //#[test]
    //fn get_cons_socket0() {}
}

//  Copyright 2020 The scaphandre authors.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
