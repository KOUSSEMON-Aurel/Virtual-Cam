use std::io;
use std::fs;

pub fn enable() -> io::Result<()> {
    // Tente d'allouer des huge pages via sysfs si elles ne sont pas déjà allouées
    // Note: Cela nécessite des privilèges root ou une configuration préalable via tuning.sh
    let path = "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages";
    if fs::metadata(path).is_ok() {
        fs::write(path, "1024")?;
    }
    Ok(())
}
