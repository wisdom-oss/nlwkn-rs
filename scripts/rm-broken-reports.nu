# delete all reports which are marked in the broken reports file
def main [broken_reports_file: path] {
  let broken_reports = (open $broken_reports_file -r | from json)
  let data_path = $broken_reports_file | path dirname
  let reports_path = $data_path | path join reports
  $broken_reports | each {|report| 
    let report_file_path = $reports_path | path join ("rep" + ($report | into string) + ".pdf")
    rm -f $report_file_path
  }

  ()
}