################################################################################
# Copyright ContinuousC. Licensed under the "Elastic License 2.0".             #
################################################################################

#!/usr/bin/env python

import sys
import winrm
import argparse
import traceback

class WinServer(object):

    WMI_METHODS = {"wmi": "Get-WmiObject", "cim": "Get-CimInstance"}

    def __init__(self, args):
        self.args = args
        self.url = "http%s://%s:%d/wsman" % (
            "s" if not args.disable_ssl else "", args.hostname,
            args.port or (5986 if not args.disable_ssl else 5985),
        )
        if args.authmethod == "basic": self.username = args.username
        elif args.authmethod == "ntlm": self.username = "%s\\%s" % (args.domain, args.username)
        elif args.authmethod == "kerberos": self.username = "%s@%s" % (args.username, args.domain)
        self.create_session()

    def create_session(self):
        print("Creating session with url: " + self.url)
        self.session = winrm.Session(
            self.url, auth = (self.username, self.args.password),
            transport  = self.args.authmethod,
            ca_trust_path = self.args.ca_cert or "legacy_requests", # legacy_requests to use environment variables
            server_cert_validation = "ignore" if self.args.danger_ignore_cert else "validate"
        )

    def __exec(self, method, cmd):
        print("Executing command: " + cmd)
        try: r = method(cmd)
        except: return sys.stderr.write("An error occured while executing command:\n%s\n" % (traceback.format_exc()))
        if r.status_code: write_err(r)
        else: print("Command successfully executed:\n" + r.std_out)

    def get_wmiobject(self, obj):
        cmd = "%s %s -Property * -ErrorAction Continue | ConvertTo-CSV -NoTypeInformation" % (WinServer.WMI_METHODS[self.args.wmi_method], obj)
        self.__exec(lambda cmd: self.session.run_ps(cmd), cmd)

    def execute_cmd(self, cmd):
        self.__exec(lambda cmd: self.session.run_cmd(cmd), cmd)
    
    def execute_ps(self, cmd):
        self.__exec(lambda cmd: self.session.run_ps(cmd), cmd)
            

def write_err(r):
    print("Command was not successfull:", r.status_code)
    print("STDOUT:\n" + r.std_out)
    print("STDERR:\n" + r.std_err)

def main(args):
    ws = WinServer(args)

    if args.wmi_object: ws.get_wmiobject(args.wmi_object)
    if args.cmd_command: ws.execute_cmd(args.cmd_command)
    if args.ps_command: ws.execute_ps(args.ps_command)

if __name__ == "__main__":
    parser = argparse.ArgumentParser("Pywinrm Test", description = "A script used to test pywinrm access")
    parser.add_argument("-H", "--hostname", required = True, help = "Name of the host")
    parser.add_argument("-P", "--port", help = "port of the winrm service. The default will be the default of the protocol used.")
    parser.add_argument("-u", "--username", required = True, help = "Name of the user used to log in")
    parser.add_argument("-p", "--password", required = True, help = "Password of the user used ot log in"),
    parser.add_argument("-m", "--authmethod", default = "ntlm", choices = {"ntlm", "basic", "kerberos"},
        help = "Authentication method used to log in. Kerberos requires a kinit before the start of this script")
    parser.add_argument("-d", "--domain", help = "Domain of the user. Required for ntlm and kerberos")
    parser.add_argument("--wmi-object", default = "Win32_Computersystem", help = "Request the following wmi object as a test")
    parser.add_argument("--wmi-method", help = "Method used to request the wmi object",
        choices = WinServer.WMI_METHODS.keys(), default = "wmi")
    parser.add_argument("--cmd-command", help = "Execute the following cmd command as a test")
    parser.add_argument("--ps-command", help = "Execute the following powershell command as a test")
    parser.add_argument("-c", "--ca-cert", help = "Use the provided certificate to verify the certiticate of the server")
    parser.add_argument("--danger-ignore-cert", action = "store_true", help = "Ignore the certificate the hosts provides")
    parser.add_argument("--disable-ssl", action = "store_true", help = "Do not use SSL and request data over HTTP")
    main(parser.parse_args())
