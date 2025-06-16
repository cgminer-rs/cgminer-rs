#!/usr/bin/env python3
"""
CGMiner-RS Monitoring Script

This script provides comprehensive monitoring of CGMiner-RS including:
- Real-time status monitoring
- Performance metrics collection
- Alert notifications
- Health checks
- Automatic reporting
"""

import argparse
import json
import time
import sys
import logging
import requests
import smtplib
from datetime import datetime, timedelta
from email.mime.text import MimeText
from email.mime.multipart import MimeMultipart
from typing import Dict, List, Optional
import websocket
import threading

class CGMinerMonitor:
    def __init__(self, base_url: str, auth_token: Optional[str] = None):
        self.base_url = base_url.rstrip('/')
        self.auth_token = auth_token
        self.headers = {'Content-Type': 'application/json'}
        if auth_token:
            self.headers['Authorization'] = f'Bearer {auth_token}'
        
        self.last_status = None
        self.alert_history = []
        self.performance_history = []
        
        # Configure logging
        logging.basicConfig(
            level=logging.INFO,
            format='%(asctime)s - %(levelname)s - %(message)s',
            handlers=[
                logging.FileHandler('cgminer_monitor.log'),
                logging.StreamHandler(sys.stdout)
            ]
        )
        self.logger = logging.getLogger(__name__)
    
    def get_status(self) -> Optional[Dict]:
        """Get current system status"""
        try:
            response = requests.get(f'{self.base_url}/api/v1/status', headers=self.headers, timeout=10)
            response.raise_for_status()
            return response.json()
        except requests.RequestException as e:
            self.logger.error(f"Failed to get status: {e}")
            return None
    
    def get_devices(self) -> Optional[List[Dict]]:
        """Get device information"""
        try:
            response = requests.get(f'{self.base_url}/api/v1/devices', headers=self.headers, timeout=10)
            response.raise_for_status()
            return response.json().get('data', [])
        except requests.RequestException as e:
            self.logger.error(f"Failed to get devices: {e}")
            return None
    
    def get_pools(self) -> Optional[List[Dict]]:
        """Get pool information"""
        try:
            response = requests.get(f'{self.base_url}/api/v1/pools', headers=self.headers, timeout=10)
            response.raise_for_status()
            return response.json().get('data', [])
        except requests.RequestException as e:
            self.logger.error(f"Failed to get pools: {e}")
            return None
    
    def check_health(self) -> Dict[str, bool]:
        """Perform comprehensive health check"""
        health = {
            'api_responsive': False,
            'mining_active': False,
            'devices_healthy': False,
            'pools_connected': False,
            'temperature_ok': False,
            'hashrate_ok': False
        }
        
        # Check API responsiveness
        status = self.get_status()
        if status and status.get('success'):
            health['api_responsive'] = True
            data = status.get('data', {})
            
            # Check mining state
            if data.get('mining_state') == 'Running':
                health['mining_active'] = True
            
            # Check hashrate
            if data.get('total_hashrate', 0) > 30.0:  # Minimum 30 GH/s
                health['hashrate_ok'] = True
        
        # Check devices
        devices = self.get_devices()
        if devices:
            healthy_devices = 0
            total_devices = len(devices)
            
            for device in devices:
                if (device.get('status') == 'Mining' and 
                    device.get('temperature', 0) < 85.0 and
                    device.get('hashrate', 0) > 15.0):
                    healthy_devices += 1
            
            if total_devices > 0 and healthy_devices / total_devices >= 0.8:
                health['devices_healthy'] = True
                health['temperature_ok'] = True
        
        # Check pools
        pools = self.get_pools()
        if pools:
            connected_pools = sum(1 for pool in pools if pool.get('status') == 'Connected')
            if connected_pools > 0:
                health['pools_connected'] = True
        
        return health
    
    def check_alerts(self, status: Dict, devices: List[Dict], pools: List[Dict]) -> List[Dict]:
        """Check for alert conditions"""
        alerts = []
        
        if not status or not status.get('success'):
            alerts.append({
                'type': 'critical',
                'message': 'API not responding',
                'timestamp': datetime.now().isoformat()
            })
            return alerts
        
        data = status.get('data', {})
        
        # Check mining state
        if data.get('mining_state') != 'Running':
            alerts.append({
                'type': 'critical',
                'message': f"Mining not active: {data.get('mining_state')}",
                'timestamp': datetime.now().isoformat()
            })
        
        # Check total hashrate
        total_hashrate = data.get('total_hashrate', 0)
        if total_hashrate < 30.0:
            alerts.append({
                'type': 'warning',
                'message': f"Low total hashrate: {total_hashrate:.1f} GH/s",
                'timestamp': datetime.now().isoformat()
            })
        
        # Check hardware errors
        hardware_errors = data.get('hardware_errors', 0)
        if hardware_errors > 10:
            alerts.append({
                'type': 'warning',
                'message': f"High hardware errors: {hardware_errors}",
                'timestamp': datetime.now().isoformat()
            })
        
        # Check device alerts
        if devices:
            for device in devices:
                device_id = device.get('device_id')
                
                # Temperature check
                temp = device.get('temperature')
                if temp and temp > 85.0:
                    alerts.append({
                        'type': 'critical',
                        'message': f"Device {device_id} high temperature: {temp:.1f}°C",
                        'timestamp': datetime.now().isoformat()
                    })
                
                # Hashrate check
                hashrate = device.get('hashrate', 0)
                if hashrate < 15.0:
                    alerts.append({
                        'type': 'warning',
                        'message': f"Device {device_id} low hashrate: {hashrate:.1f} GH/s",
                        'timestamp': datetime.now().isoformat()
                    })
                
                # Status check
                if device.get('status') != 'Mining':
                    alerts.append({
                        'type': 'critical',
                        'message': f"Device {device_id} not mining: {device.get('status')}",
                        'timestamp': datetime.now().isoformat()
                    })
        
        # Check pool alerts
        if pools:
            connected_pools = [p for p in pools if p.get('status') == 'Connected']
            if not connected_pools:
                alerts.append({
                    'type': 'critical',
                    'message': "No pools connected",
                    'timestamp': datetime.now().isoformat()
                })
        
        return alerts
    
    def send_email_alert(self, alerts: List[Dict], smtp_config: Dict):
        """Send email alerts"""
        if not alerts or not smtp_config:
            return
        
        try:
            msg = MimeMultipart()
            msg['From'] = smtp_config['from']
            msg['To'] = smtp_config['to']
            msg['Subject'] = f"CGMiner-RS Alert - {len(alerts)} issues detected"
            
            body = "CGMiner-RS Alert Report\n"
            body += "=" * 30 + "\n\n"
            
            for alert in alerts:
                body += f"[{alert['type'].upper()}] {alert['message']}\n"
                body += f"Time: {alert['timestamp']}\n\n"
            
            msg.attach(MimeText(body, 'plain'))
            
            server = smtplib.SMTP(smtp_config['server'], smtp_config['port'])
            if smtp_config.get('use_tls'):
                server.starttls()
            if smtp_config.get('username'):
                server.login(smtp_config['username'], smtp_config['password'])
            
            server.send_message(msg)
            server.quit()
            
            self.logger.info(f"Email alert sent with {len(alerts)} issues")
            
        except Exception as e:
            self.logger.error(f"Failed to send email alert: {e}")
    
    def collect_performance_metrics(self):
        """Collect and store performance metrics"""
        status = self.get_status()
        devices = self.get_devices()
        
        if status and status.get('success'):
            data = status.get('data', {})
            
            metrics = {
                'timestamp': datetime.now().isoformat(),
                'total_hashrate': data.get('total_hashrate', 0),
                'accepted_shares': data.get('accepted_shares', 0),
                'rejected_shares': data.get('rejected_shares', 0),
                'hardware_errors': data.get('hardware_errors', 0),
                'active_devices': data.get('active_devices', 0),
                'uptime': data.get('uptime', 0)
            }
            
            if devices:
                metrics['device_temperatures'] = [d.get('temperature', 0) for d in devices if d.get('temperature')]
                metrics['device_hashrates'] = [d.get('hashrate', 0) for d in devices if d.get('hashrate')]
            
            self.performance_history.append(metrics)
            
            # Keep only last 24 hours of data
            cutoff_time = datetime.now() - timedelta(hours=24)
            self.performance_history = [
                m for m in self.performance_history 
                if datetime.fromisoformat(m['timestamp']) > cutoff_time
            ]
    
    def generate_report(self) -> str:
        """Generate performance report"""
        if not self.performance_history:
            return "No performance data available"
        
        latest = self.performance_history[-1]
        
        report = "CGMiner-RS Performance Report\n"
        report += "=" * 35 + "\n\n"
        
        report += f"Report Time: {latest['timestamp']}\n"
        report += f"Total Hashrate: {latest['total_hashrate']:.1f} GH/s\n"
        report += f"Accepted Shares: {latest['accepted_shares']}\n"
        report += f"Rejected Shares: {latest['rejected_shares']}\n"
        report += f"Hardware Errors: {latest['hardware_errors']}\n"
        report += f"Active Devices: {latest['active_devices']}\n"
        report += f"Uptime: {latest['uptime']} seconds\n\n"
        
        if 'device_temperatures' in latest:
            temps = latest['device_temperatures']
            if temps:
                report += f"Device Temperatures: {min(temps):.1f}°C - {max(temps):.1f}°C (avg: {sum(temps)/len(temps):.1f}°C)\n"
        
        if 'device_hashrates' in latest:
            hashrates = latest['device_hashrates']
            if hashrates:
                report += f"Device Hashrates: {min(hashrates):.1f} - {max(hashrates):.1f} GH/s (avg: {sum(hashrates)/len(hashrates):.1f} GH/s)\n"
        
        # Calculate efficiency metrics
        if len(self.performance_history) > 1:
            report += "\n24-Hour Trends:\n"
            report += "-" * 15 + "\n"
            
            first = self.performance_history[0]
            shares_gained = latest['accepted_shares'] - first['accepted_shares']
            time_diff = (datetime.fromisoformat(latest['timestamp']) - 
                        datetime.fromisoformat(first['timestamp'])).total_seconds() / 3600
            
            if time_diff > 0:
                shares_per_hour = shares_gained / time_diff
                report += f"Shares per hour: {shares_per_hour:.1f}\n"
        
        return report
    
    def monitor_continuous(self, interval: int = 60, smtp_config: Optional[Dict] = None):
        """Run continuous monitoring"""
        self.logger.info(f"Starting continuous monitoring (interval: {interval}s)")
        
        while True:
            try:
                # Get current status
                status = self.get_status()
                devices = self.get_devices()
                pools = self.get_pools()
                
                # Check for alerts
                alerts = self.check_alerts(status, devices or [], pools or [])
                
                # Send alerts if any
                if alerts:
                    self.logger.warning(f"Found {len(alerts)} alerts")
                    for alert in alerts:
                        self.logger.warning(f"[{alert['type']}] {alert['message']}")
                    
                    if smtp_config:
                        self.send_email_alert(alerts, smtp_config)
                
                # Collect performance metrics
                self.collect_performance_metrics()
                
                # Log status
                if status and status.get('success'):
                    data = status.get('data', {})
                    self.logger.info(
                        f"Status: {data.get('mining_state')} | "
                        f"Hashrate: {data.get('total_hashrate', 0):.1f} GH/s | "
                        f"Devices: {data.get('active_devices', 0)} | "
                        f"Shares: {data.get('accepted_shares', 0)}"
                    )
                
                time.sleep(interval)
                
            except KeyboardInterrupt:
                self.logger.info("Monitoring stopped by user")
                break
            except Exception as e:
                self.logger.error(f"Monitoring error: {e}")
                time.sleep(interval)

def main():
    parser = argparse.ArgumentParser(description='CGMiner-RS Monitoring Script')
    parser.add_argument('--url', default='http://localhost:8080', help='CGMiner-RS API URL')
    parser.add_argument('--token', help='Authentication token')
    parser.add_argument('--interval', type=int, default=60, help='Monitoring interval in seconds')
    parser.add_argument('--mode', choices=['status', 'health', 'monitor', 'report'], 
                       default='status', help='Operation mode')
    parser.add_argument('--smtp-server', help='SMTP server for alerts')
    parser.add_argument('--smtp-port', type=int, default=587, help='SMTP port')
    parser.add_argument('--smtp-user', help='SMTP username')
    parser.add_argument('--smtp-pass', help='SMTP password')
    parser.add_argument('--smtp-from', help='From email address')
    parser.add_argument('--smtp-to', help='To email address')
    
    args = parser.parse_args()
    
    monitor = CGMinerMonitor(args.url, args.token)
    
    if args.mode == 'status':
        status = monitor.get_status()
        if status:
            print(json.dumps(status, indent=2))
        else:
            print("Failed to get status")
            sys.exit(1)
    
    elif args.mode == 'health':
        health = monitor.check_health()
        print("Health Check Results:")
        print("=" * 20)
        for check, result in health.items():
            status = "✓" if result else "✗"
            print(f"{status} {check.replace('_', ' ').title()}")
        
        if all(health.values()):
            print("\nOverall Status: HEALTHY")
            sys.exit(0)
        else:
            print("\nOverall Status: UNHEALTHY")
            sys.exit(1)
    
    elif args.mode == 'report':
        monitor.collect_performance_metrics()
        report = monitor.generate_report()
        print(report)
    
    elif args.mode == 'monitor':
        smtp_config = None
        if args.smtp_server and args.smtp_from and args.smtp_to:
            smtp_config = {
                'server': args.smtp_server,
                'port': args.smtp_port,
                'username': args.smtp_user,
                'password': args.smtp_pass,
                'from': args.smtp_from,
                'to': args.smtp_to,
                'use_tls': True
            }
        
        monitor.monitor_continuous(args.interval, smtp_config)

if __name__ == '__main__':
    main()
