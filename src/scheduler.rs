// ============================================================================
// Prism Visuals - Windows Task Scheduler Integration
// ============================================================================
// This module handles automatic wallpaper scheduling using Windows Task Scheduler.


use std::process::Command;
use std::path::PathBuf;

/// Task Scheduler configuration for auto-change
pub struct SchedulerConfig {
    pub task_name: String,
    pub exe_path: PathBuf,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        // Get the path to the current executable
        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("visuals.exe"));
        
        SchedulerConfig {
            task_name: "PrismVisuals-AutoChange".to_string(),
            exe_path,
        }
    }
}

/// Frequency options for auto-change scheduling
#[derive(Debug, Clone, PartialEq)]
pub enum ScheduleFrequency {
    AutoDaily,                    // Auto daily at 8:00 AM (default, zero-config)
    Daily { time: String },       // User-specified time e.g., "09:00"
    Hourly,                       // Every hour
    Hours3,                       // Every 3 hours
    Hours6,                       // Every 6 hours
    Custom { hours: u32 },        // Custom interval in hours
    Minute1Test,                  // TEST ONLY: Every 1 minute (for flicker testing)
}

impl ScheduleFrequency {
    /// Get the repetition interval in ISO 8601 duration format
    pub fn to_interval(&self) -> String {
        match self {
            ScheduleFrequency::AutoDaily => "P1D".to_string(),     // 1 day
            ScheduleFrequency::Daily { .. } => "P1D".to_string(),  // 1 day
            ScheduleFrequency::Hourly => "PT1H".to_string(),       // 1 hour
            ScheduleFrequency::Hours3 => "PT3H".to_string(),       // 3 hours
            ScheduleFrequency::Hours6 => "PT6H".to_string(),       // 6 hours
            ScheduleFrequency::Custom { hours } => format!("PT{}H", hours),
            ScheduleFrequency::Minute1Test => "PT1M".to_string(), // 1 minute (test)
        }
    }

    /// Get human-readable display string
    pub fn display(&self) -> String {
        match self {
            ScheduleFrequency::AutoDaily => "Auto Daily at 8:00 AM".to_string(),
            ScheduleFrequency::Daily { time } => format!("Daily at {}", time),
            ScheduleFrequency::Hourly => "Every hour".to_string(),
            ScheduleFrequency::Hours3 => "Every 3 hours".to_string(),
            ScheduleFrequency::Hours6 => "Every 6 hours".to_string(),
            ScheduleFrequency::Custom { hours } => format!("Every {} hours", hours),
            ScheduleFrequency::Minute1Test => "TEST: Every 1 minute".to_string(),
        }
    }

    /// Convert to config string for storage
    pub fn to_config_string(&self) -> String {
        match self {
            ScheduleFrequency::AutoDaily => "auto_daily".to_string(),
            ScheduleFrequency::Daily { time } => format!("daily:{}", time),
            ScheduleFrequency::Hourly => "hourly".to_string(),
            ScheduleFrequency::Hours3 => "3hours".to_string(),
            ScheduleFrequency::Hours6 => "6hours".to_string(),
            ScheduleFrequency::Custom { hours } => format!("custom:{}", hours),
            ScheduleFrequency::Minute1Test => "test_1m".to_string(),
        }
    }

    /// Parse from config string
    pub fn from_config_string(s: &str) -> Option<Self> {
        if s == "auto_daily" {
            Some(ScheduleFrequency::AutoDaily)
        } else if s.starts_with("daily:") {
            let time = s.strip_prefix("daily:")?.to_string();
            Some(ScheduleFrequency::Daily { time })
        } else if s == "hourly" {
            Some(ScheduleFrequency::Hourly)
        } else if s == "3hours" {
            Some(ScheduleFrequency::Hours3)
        } else if s == "6hours" {
            Some(ScheduleFrequency::Hours6)
        } else if s.starts_with("custom:") {
            let hours = s.strip_prefix("custom:")?.parse().ok()?;
            Some(ScheduleFrequency::Custom { hours })
        } else if s == "test_1m" || s == "test_10s" {
            Some(ScheduleFrequency::Minute1Test)
        } else {
            None
        }
    }
}

/// Windows Task Scheduler manager using schtasks.exe command
/// This approach is more reliable than COM API and doesn't require additional dependencies
pub struct TaskScheduler {
    config: SchedulerConfig,
}

impl TaskScheduler {
    pub fn new() -> Self {
        TaskScheduler {
            config: SchedulerConfig::default(),
        }
    }

    /// Create a scheduled task for auto-changing wallpapers
    /// Uses schtasks.exe which is built into Windows - no extra deps needed
    pub fn create_task(&self, frequency: &ScheduleFrequency) -> Result<(), String> {
        // First, delete any existing task and VBS wrapper
        let _ = self.delete_task();

        // Build the schtasks command based on frequency
        let exe_path = self.config.exe_path.to_string_lossy();
        let task_name = &self.config.task_name;

       
        // If this fails, return special error for UAC elevation
        if let Err(e) = self.create_vbs_wrapper(&exe_path) {
            return Err(format!("NEEDS_ELEVATION:{}", e));
        }

        // Create XML for the scheduled task (more flexible than command-line options)
        let xml = self.generate_task_xml(frequency, &exe_path);
        
        // Write XML to temp file
        let temp_dir = std::env::temp_dir();
        let xml_path = temp_dir.join("prism_visuals_task.xml");
        
        std::fs::write(&xml_path, &xml)
            .map_err(|e| format!("Failed to write task XML: {}", e))?;

        // Create task from XML
        let output = Command::new("schtasks")
            .args([
                "/Create",
                "/TN", task_name,
                "/XML", &xml_path.to_string_lossy(),
                "/F",  // Force overwrite
            ])
            .output()
            .map_err(|e| format!("Failed to run schtasks: {}", e))?;

        // Clean up temp file
        let _ = std::fs::remove_file(&xml_path);

        if output.status.success() {
            Ok(())
        } else {
            // If task creation fails, clean up VBS wrapper
            self.delete_vbs_wrapper();
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to create scheduled task: {}", stderr))
        }
    }

    /// Generate XML configuration for the scheduled task
    /// Uses VBScript wrapper for completely silent execution (no window flicker)
    fn generate_task_xml(&self, frequency: &ScheduleFrequency, exe_path: &str) -> String {
        let now = chrono::Local::now();
        
        // Calculate start time based on frequency
        let start_time = match frequency {
            ScheduleFrequency::AutoDaily => {
                // Auto Daily: hardcoded to 8:00 AM
                let today = now.date_naive();
                format!("{}T08:00:00", today)
            }
            ScheduleFrequency::Daily { time } => {
                // Parse the time and create start boundary
                let parts: Vec<&str> = time.split(':').collect();
                let hour: u32 = parts.get(0).and_then(|h| h.parse().ok()).unwrap_or(9);
                let minute: u32 = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);
                
                // Use tomorrow's date if time has passed today
                let today = now.date_naive();
                format!("{}T{:02}:{:02}:00", today, hour, minute)
            }
            ScheduleFrequency::Minute1Test => {
                // Start immediately for testing
                now.format("%Y-%m-%dT%H:%M:%S").to_string()
            }
            _ => {
                // For interval-based, start from next hour
                let next_hour = now + chrono::Duration::hours(1);
                next_hour.format("%Y-%m-%dT%H:00:00").to_string()
            }
        };

        let interval = frequency.to_interval();
        
        // For daily tasks, we use CalendarTrigger; for hourly/seconds, we use repetition
        let trigger_xml = match frequency {
            ScheduleFrequency::AutoDaily | ScheduleFrequency::Daily { .. } => {
                format!(r#"
    <CalendarTrigger>
      <StartBoundary>{}</StartBoundary>
      <Enabled>true</Enabled>
      <ScheduleByDay>
        <DaysInterval>1</DaysInterval>
      </ScheduleByDay>
    </CalendarTrigger>"#, start_time)
            }
            _ => {
                format!(r#"
    <TimeTrigger>
      <StartBoundary>{}</StartBoundary>
      <Enabled>true</Enabled>
      <Repetition>
        <Interval>{}</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
    </TimeTrigger>"#, start_time, interval)
            }
        };

        // Get VBS path for completely silent execution
        let vbs_path = self.get_vbs_path();
        let vbs_path_str = vbs_path.to_string_lossy();

        // Task uses wscript.exe to run VBS in completely hidden mode
        format!(r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Prism Visuals Auto-Change Wallpaper</Description>
    <Author>Prism Visuals</Author>
  </RegistrationInfo>
  <Triggers>{trigger_xml}
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT10M</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>wscript.exe</Command>
      <Arguments>"{vbs_path_str}" //B //Nologo</Arguments>
    </Exec>
  </Actions>
</Task>"#)
    }

    /// Create VBScript wrapper for completely silent execution
    /// Returns the path to the VBS file
    fn create_vbs_wrapper(&self, exe_path: &str) -> Result<std::path::PathBuf, String> {
        let vbs_path = self.get_vbs_path();
        
        // VBScript content: Run command with window style 0 (completely hidden)
        let vbs_content = format!(
            r#"Set objShell = CreateObject("WScript.Shell")
objShell.Run """{}"" auto-change", 0, False
"#,
            exe_path
        );
        
        std::fs::write(&vbs_path, vbs_content)
            .map_err(|e| format!("Failed to create VBS wrapper: {}", e))?;
        
        Ok(vbs_path)
    }

    /// Get path to VBS wrapper file (in user's AppData folder for no UAC requirement)
    fn get_vbs_path(&self) -> std::path::PathBuf {
        // Store VBS in user's AppData folder (always writable, no UAC needed)
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let prism_dir = std::path::PathBuf::from(appdata).join("Prism Visuals");
            // Create directory if it doesn't exist
            let _ = std::fs::create_dir_all(&prism_dir);
            prism_dir.join("prism_auto_change.vbs")
        } else {
            // Fallback to exe directory (may require admin)
            self.config.exe_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("prism_auto_change.vbs")
        }
    }

    /// Delete VBS wrapper file
    fn delete_vbs_wrapper(&self) {
        let vbs_path = self.get_vbs_path();
        let _ = std::fs::remove_file(vbs_path);
    }

    /// Delete the scheduled task and VBS wrapper
    pub fn delete_task(&self) -> Result<(), String> {
        // Delete VBS wrapper file
        self.delete_vbs_wrapper();

        let output = Command::new("schtasks")
            .args([
                "/Delete",
                "/TN", &self.config.task_name,
                "/F",  // Force delete without confirmation
            ])
            .output()
            .map_err(|e| format!("Failed to run schtasks: {}", e))?;

        // Ignore errors if task doesn't exist
        if output.status.success() || String::from_utf8_lossy(&output.stderr).contains("does not exist") {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to delete scheduled task: {}", stderr))
        }
    }

    /// Check if scheduled task exists and is enabled
    pub fn task_exists(&self) -> bool {
        let output = Command::new("schtasks")
            .args([
                "/Query",
                "/TN", &self.config.task_name,
                "/FO", "LIST",
            ])
            .output();

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    /// Get task info (returns next run time if task exists)
    pub fn get_task_info(&self) -> Option<TaskInfo> {
        let output = Command::new("schtasks")
            .args([
                "/Query",
                "/TN", &self.config.task_name,
                "/FO", "LIST",
                "/V",  // Verbose
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse the output to extract info
        let mut next_run = String::new();
        let mut last_run = String::new();
        let mut status = String::new();

        for line in stdout.lines() {
            if line.starts_with("Next Run Time:") {
                next_run = line.replace("Next Run Time:", "").trim().to_string();
            } else if line.starts_with("Last Run Time:") {
                last_run = line.replace("Last Run Time:", "").trim().to_string();
            } else if line.starts_with("Status:") {
                status = line.replace("Status:", "").trim().to_string();
            }
        }

        Some(TaskInfo {
            next_run,
            last_run,
            status,
        })
    }
}

/// Information about a scheduled task
#[derive(Debug)]
pub struct TaskInfo {
    pub next_run: String,
    pub last_run: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_to_interval() {
        assert_eq!(ScheduleFrequency::Hourly.to_interval(), "PT1H");
        assert_eq!(ScheduleFrequency::Hours3.to_interval(), "PT3H");
        assert_eq!(ScheduleFrequency::Hours6.to_interval(), "PT6H");
        assert_eq!(ScheduleFrequency::Daily { time: "09:00".to_string() }.to_interval(), "P1D");
        assert_eq!(ScheduleFrequency::Custom { hours: 2 }.to_interval(), "PT2H");
    }

    #[test]
    fn test_frequency_config_roundtrip() {
        let freqs = vec![
            ScheduleFrequency::AutoDaily,
            ScheduleFrequency::Daily { time: "09:00".to_string() },
            ScheduleFrequency::Hourly,
            ScheduleFrequency::Hours3,
            ScheduleFrequency::Hours6,
            ScheduleFrequency::Custom { hours: 4 },
        ];

        for freq in freqs {
            let config_str = freq.to_config_string();
            let parsed = ScheduleFrequency::from_config_string(&config_str);
            assert_eq!(parsed, Some(freq));
        }
    }
}
