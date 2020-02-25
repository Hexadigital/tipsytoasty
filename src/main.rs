use std::env;
use std::error::Error;
use std::process;
use std::ffi::OsString;
use std::fs::File;
use std::collections::HashMap;

// Alias our "row" construct that we'll use to represent the data
type Row = HashMap<String, String>;

fn parse() -> Result<(), Box<dyn Error>> {
	// TODO: get minimum wage from config file
	let minwage = 9.65;
	// Create a vector to hold the employee data we care about
	let mut tipped_shifts = Vec::<Row>::new();
	
	// Get the path to the CSV from the argument passed to the program
	let file_path = get_first_arg()?;
	// Open the CSV
	let file = File::open(&file_path)?;
    // Build the CSV reader using our file
    let mut rdr = csv::Reader::from_reader(file);
	// Loop through rows
    for result in rdr.deserialize() {
		// Build another HashMap for storing the data we care about
		let mut work_data = Row::new();
		
		// Map row data to column headers
		let record: Row = result?;
		// Reformat full name
		let name: Vec<&str> = record["Employee"].split(", ").collect();
		work_data.insert(String::from("Full Name"), name[1].to_owned() + " " + name[0]);
		
		// Find date worked
		let date: Vec<&str> = record["In Date"].split(" ").collect();
		// TODO: Check for dupes
		work_data.insert(String::from("Date"), date[0].to_string());
		
		// Find hours worked
		let hours: f32 = record["Payable Hours"].parse().unwrap();
		work_data.insert(String::from("Hours Worked"), hours.to_string());
		
		// Find wage earned
		let pay: f32 = record["Total Pay"].parse().unwrap();
		work_data.insert(String::from("Wages Paid"), pay.to_string());
		
		// Find tips earned
		let tips: f32 = record["Total Tips"].parse().unwrap();
		work_data.insert(String::from("Tips"), tips.to_string());
		
		// Is this a tipped wage shift?
		let hourlywage: f32 = record["Wage"].parse().unwrap();
		if hourlywage < minwage {
			// It's a tipped shift!
			tipped_shifts.push(work_data);
		}
    }
	
	// Now let's loop through the shifts we care about, and combine them by date
	let mut daywork = HashMap::<(String, String), (f32, f32, f32)>::new();
	for shift in &tipped_shifts {
		// TODO: Consolidate this with the above code
		let whowhen = (shift["Full Name"].to_owned(), shift["Date"].to_owned());
		let time: f32 = shift["Hours Worked"].parse().unwrap();
		let wage: f32 = shift["Wages Paid"].parse().unwrap();
		let tip: f32 = shift["Tips"].parse().unwrap();
		let timewagetip = (time, wage, tip);
		// Do we already have a shift for this employee on this date?
		if daywork.contains_key(&whowhen) {
			// Add the shifts together
			let oldtuple = daywork[&whowhen];
			let newtuple = (oldtuple.0+time, oldtuple.1+wage, oldtuple.2+tip);
			daywork.insert(whowhen, newtuple);
		} else {
			// Add the new shift
			daywork.insert(whowhen, timewagetip);
		}
	}
	
	// And reiterate to combine by employee
	let mut totalwork = HashMap::<String, (f32, f32, f32, f32)>::new();
	for (whowhen, timewagetip) in &daywork {
		let who = whowhen.0.to_string();
		let time: f32 = timewagetip.0;
		let wage: f32 = timewagetip.1;
		let tip: f32 = timewagetip.2;
		
		let minpay: f32 = time * minwage;
		let actualpay: f32 = wage + tip;
		let difference2: f32 = &minpay - &actualpay;
		// If the employee is making more than minimum wage, we don't care
		let diffcare = difference2.max(0.00);

		let timewagetipdiff = (time, wage, tip, diffcare);
		println!("{:?}", timewagetipdiff);
		// Do we already have a shift for this employee?
		if totalwork.contains_key(&who) {
			// Add the shifts together
			let oldtuple = totalwork[&who];
			let newtuple = (oldtuple.0+time, oldtuple.1+wage, oldtuple.2+tip, oldtuple.3+diffcare);
			totalwork.insert(who.to_string(), newtuple);
		} else {
			// Add the new shift
			totalwork.insert(who.to_string(), timewagetipdiff);
		}
	}
	
	// Nasty path work
	let path = &file_path.to_str().unwrap();
	let mut path_split: Vec<&str> = path.split("/").collect(); // TODO: Linux compatability
	// get the file name with extension
	let file_name = path_split[path_split.len() - 1];
	// get the file extension
	let file_extension_temp: Vec<&str> = file_name.split(".").collect();
	let file_extension = file_extension_temp[file_extension_temp.len() - 1];
	// strip extension from file name
	let file_name = file_extension_temp[0];
	// strip file name from our path vector
	path_split.pop();
	// build our path string from the vector
	let mut file_folder = "".to_string();
	for slice in path_split {
		file_folder += slice;
		file_folder += "//"; // TODO: Linux compatability
	}
	
	// Build path for daily report
	let mut daily_path = file_folder.to_string();
	daily_path += &file_name;
	daily_path += "-daily.";
	daily_path += &file_extension;
	println!("{}", daily_path);
	
	// Build path for total report
	let mut total_path = file_folder.to_string();
	total_path += &file_name;
	total_path += "-total.";
	total_path += &file_extension;
	println!("{}", total_path);
	
	// Begin export to CSV
	let mut wtr = csv::Writer::from_path(daily_path)?;
	wtr.write_record(&["Name", "Date", "Hours Worked", "Wages Paid", "Tips Made", "Difference Needed", "Difference"])?;
	
	// Now, let's finally loop through the dates and calculate wages for daily
	for (whowhen, timewagetip) in &daywork {
		let minpay = &timewagetip.0 * minwage;
		let actualpay = &timewagetip.1 + &timewagetip.2;
		if &actualpay < &minpay {
			let difference = &minpay - &actualpay;
			print!("{}", "On ".to_owned() + &whowhen.1 + ", " + &whowhen.0 + " needed to be paid $");
			print!("{:.2}", &minpay);
			print!("{}", " but instead got paid $");
			print!("{:.2}", &actualpay);
			println!("{}", ".");
			wtr.write_record(&[&whowhen.0.to_string(), &whowhen.1.to_string(), &format!("{:.2}", &timewagetip.0), &format!("{:.2}", &timewagetip.1), &format!("{:.2}", &timewagetip.2), "YES", &format!("{:.2}", &difference)])?;
		} else {
			wtr.write_record(&[&whowhen.0.to_string(), &whowhen.1.to_string(), &format!("{:.2}", &timewagetip.0), &format!("{:.2}", timewagetip.1), &format!("{:.2}", &timewagetip.2), "NO", "0"])?;
		}
		
	}
	wtr.flush()?;
	
	// now repeat for total
	let mut wtr = csv::Writer::from_path(total_path)?;
	wtr.write_record(&["Name", "Hours Worked", "Wages Paid", "Tips Made", "Difference"])?;
	
	// TODO: Display warning if tipped employee worked over 40 hours
	for (who, timewagetip) in &totalwork {
		wtr.write_record(&[&who.to_string(), &format!("{:.2}", &timewagetip.0), &format!("{:.2}", &timewagetip.1), &format!("{:.2}", &timewagetip.2), &format!("{:.2}", &timewagetip.3)])?;
	}
	wtr.flush()?;
	
    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = parse() {
        println!("error: {}", err);
        process::exit(1);
    }
}
