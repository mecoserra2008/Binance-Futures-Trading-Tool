#!/bin/bash

# Binance Futures Orderflow Screener - Health Check Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROCESS_NAME="binance-screener"
DB_FILE="data.db"
CONFIG_FILE="config.toml"
LOG_DIR="logs"
MAX_LOG_SIZE_MB=100
MAX_DB_SIZE_GB=5

# Health check functions
check_process() {
    echo -e "${BLUE}🔍 Checking process status...${NC}"
    
    if pgrep -f "$PROCESS_NAME" > /dev/null; then
        PID=$(pgrep -f "$PROCESS_NAME")
        CPU_USAGE=$(ps -p $PID -o %cpu --no-headers | tr -d ' ')
        MEM_USAGE=$(ps -p $PID -o %mem --no-headers | tr -d ' ')
        
        echo -e "${GREEN}✅ Process is running${NC}"
        echo "   PID: $PID"
        echo "   CPU: ${CPU_USAGE}%"
        echo "   Memory: ${MEM_USAGE}%"
        return 0
    else
        echo -e "${RED}❌ Process is not running${NC}"
        return 1
    fi
}

check_database() {
    echo -e "${BLUE}🗄️  Checking database...${NC}"
    
    if [ -f "$DB_FILE" ]; then
        DB_SIZE=$(du -h "$DB_FILE" | cut -f1)
        DB_SIZE_BYTES=$(stat -f%z "$DB_FILE" 2>/dev/null || stat -c%s "$DB_FILE" 2>/dev/null || echo "0")
        DB_SIZE_GB=$((DB_SIZE_BYTES / 1024 / 1024 / 1024))
        
        echo -e "${GREEN}✅ Database file exists${NC}"
        echo "   Size: $DB_SIZE"
        
        # Check if database is too large
        if [ $DB_SIZE_GB -gt $MAX_DB_SIZE_GB ]; then
            echo -e "${YELLOW}⚠️  Warning: Database is large (${DB_SIZE_GB}GB > ${MAX_DB_SIZE_GB}GB)${NC}"
        fi
        
        # Try to query database
        if command -v sqlite3 &> /dev/null; then
            TABLES=$(sqlite3 "$DB_FILE" ".tables" 2>/dev/null | wc -w)
            echo "   Tables: $TABLES"
            
            # Check for recent data
            RECENT_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM raw_trades WHERE timestamp > $(date -d '1 hour ago' +%s)000;" 2>/dev/null || echo "N/A")
            echo "   Recent trades (1h): $RECENT_COUNT"
        fi
        
        return 0
    else
        echo -e "${RED}❌ Database file not found${NC}"
        return 1
    fi
}

check_config() {
    echo -e "${BLUE}⚙️  Checking configuration...${NC}"
    
    if [ -f "$CONFIG_FILE" ]; then
        echo -e "${GREEN}✅ Config file exists${NC}"
        
        # Check config file size and modification time
        CONFIG_SIZE=$(du -h "$CONFIG_FILE" | cut -f1)
        CONFIG_MTIME=$(stat -f%Sm "$CONFIG_FILE" 2>/dev/null || stat -c%y "$CONFIG_FILE" 2>/dev/null || echo "Unknown")
        
        echo "   Size: $CONFIG_SIZE"
        echo "   Modified: $CONFIG_MTIME"
        
        return 0
    else
        echo -e "${RED}❌ Config file not found${NC}"
        return 1
    fi
}

check_logs() {
    echo -e "${BLUE}📋 Checking logs...${NC}"
    
    if [ -d "$LOG_DIR" ]; then
        LOG_COUNT=$(find "$LOG_DIR" -name "*.log" 2>/dev/null | wc -l)
        echo -e "${GREEN}✅ Log directory exists${NC}"
        echo "   Log files: $LOG_COUNT"
        
        # Check log sizes
        for log_file in "$LOG_DIR"/*.log; do
            if [ -f "$log_file" ]; then
                LOG_SIZE_MB=$(du -m "$log_file" | cut -f1)
                LOG_NAME=$(basename "$log_file")
                
                if [ $LOG_SIZE_MB -gt $MAX_LOG_SIZE_MB ]; then
                    echo -e "${YELLOW}⚠️  Warning: Large log file: $LOG_NAME (${LOG_SIZE_MB}MB)${NC}"
                fi
                
                # Check for recent errors
                ERROR_COUNT=$(grep -c "ERROR\|FATAL" "$log_file" 2>/dev/null | tail -1 || echo "0")
                if [ $ERROR_COUNT -gt 0 ]; then
                    echo -e "${YELLOW}⚠️  Errors found in $LOG_NAME: $ERROR_COUNT${NC}"
                fi
            fi
        done
        
        return 0
    else
        echo -e "${YELLOW}⚠️  Log directory not found${NC}"
        return 1
    fi
}

check_network() {
    echo -e "${BLUE}🌐 Checking network connectivity...${NC}"
    
    # Test Binance API connectivity
    if curl -s --max-time 10 "https://fapi.binance.com/fapi/v1/ping" > /dev/null; then
        echo -e "${GREEN}✅ Binance API reachable${NC}"
    else
        echo -e "${RED}❌ Cannot reach Binance API${NC}"
        return 1
    fi
    
    # Test DNS resolution
    if nslookup fstream.binance.com > /dev/null 2>&1; then
        echo -e "${GREEN}✅ DNS resolution working${NC}"
    else
        echo -e "${RED}❌ DNS resolution failed${NC}"
        return 1
    fi
    
    return 0
}

check_system_resources() {
    echo -e "${BLUE}💻 Checking system resources...${NC}"
    
    # Memory usage
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        MEMORY_USED=$(free | awk 'NR==2{printf "%.1f", $3/$2*100}')
        DISK_USED=$(df . | awk 'NR==2{print $5}' | sed 's/%//')
        
        echo "   Memory usage: ${MEMORY_USED}%"
        echo "   Disk usage: ${DISK_USED}%"
        
        if (( $(echo "$MEMORY_USED > 90" | bc -l) )); then
            echo -e "${YELLOW}⚠️  High memory usage${NC}"
        fi
        
        if [ $DISK_USED -gt 90 ]; then
            echo -e "${YELLOW}⚠️  High disk usage${NC}"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        DISK_USED=$(df . | awk 'NR==2{print $5}' | sed 's/%//')
        echo "   Disk usage: ${DISK_USED}%"
        
        if [ $DISK_USED -gt 90 ]; then
            echo -e "${YELLOW}⚠️  High disk usage${NC}"
        fi
    fi
    
    return 0
}

# Performance metrics
check_performance() {
    echo -e "${BLUE}⚡ Checking performance metrics...${NC}"
    
    if [ -f "$DB_FILE" ] && command -v sqlite3 &> /dev/null; then
        # Database metrics
        TOTAL_TRADES=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM raw_trades;" 2>/dev/null || echo "N/A")
        TRADES_PER_MINUTE=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM raw_trades WHERE timestamp > $(date -d '1 minute ago' +%s)000;" 2>/dev/null || echo "N/A")
        
        echo "   Total trades: $TOTAL_TRADES"
        echo "   Trades/min: $TRADES_PER_MINUTE"
        
        # Check if processing is keeping up
        if [ "$TRADES_PER_MINUTE" != "N/A" ] && [ $TRADES_PER_MINUTE -lt 10 ]; then
            echo -e "${YELLOW}⚠️  Low trading activity or processing lag${NC}"
        fi
    fi
    
    return 0
}

# Cleanup recommendations
suggest_maintenance() {
    echo -e "${BLUE}🧹 Maintenance suggestions...${NC}"
    
    # Check for old log files
    OLD_LOGS=$(find "$LOG_DIR" -name "*.log" -mtime +7 2>/dev/null | wc -l)
    if [ $OLD_LOGS -gt 0 ]; then
        echo -e "${YELLOW}💡 Consider rotating old log files (${OLD_LOGS} files older than 7 days)${NC}"
    fi
    
    # Check backup age
    if [ -d "backups" ]; then
        LATEST_BACKUP=$(find backups -name "*.db" -type f -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2)
        if [ -n "$LATEST_BACKUP" ]; then
            BACKUP_AGE=$(( ($(date +%s) - $(stat -c%Y "$LATEST_BACKUP" 2>/dev/null || echo "0")) / 86400 ))
            if [ $BACKUP_AGE -gt 7 ]; then
                echo -e "${YELLOW}💡 Latest backup is ${BACKUP_AGE} days old - consider creating fresh backup${NC}"
            fi
        else
            echo -e "${YELLOW}💡 No database backups found - consider running 'make backup'${NC}"
        fi
    fi
}

# Main health check
main() {
    echo -e "${BLUE}🏥 Binance Screener Health Check${NC}"
    echo "=================================="
    echo ""
    
    OVERALL_STATUS=0
    
    check_process || OVERALL_STATUS=1
    echo ""
    
    check_database || OVERALL_STATUS=1
    echo ""
    
    check_config || OVERALL_STATUS=1
    echo ""
    
    check_logs || OVERALL_STATUS=1
    echo ""
    
    check_network || OVERALL_STATUS=1
    echo ""
    
    check_system_resources
    echo ""
    
    check_performance
    echo ""
    
    suggest_maintenance
    echo ""
    
    # Overall status
    if [ $OVERALL_STATUS -eq 0 ]; then
        echo -e "${GREEN}🎉 Overall Status: HEALTHY${NC}"
    else
        echo -e "${RED}🚨 Overall Status: ISSUES DETECTED${NC}"
    fi
    
    return $OVERALL_STATUS
}

# Command line options
case "${1:-all}" in
    "all")
        main
        ;;
    "process")
        check_process
        ;;
    "database")
        check_database
        ;;
    "config")
        check_config
        ;;
    "logs")
        check_logs
        ;;
    "network")
        check_network
        ;;
    "resources")
        check_system_resources
        ;;
    "performance")
        check_performance
        ;;
    *)
        echo "Usage: $0 [all|process|database|config|logs|network|resources|performance]"
        exit 1
        ;;
esac