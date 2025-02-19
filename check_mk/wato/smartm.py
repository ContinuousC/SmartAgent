################################################################################
# Copyright ContinuousC. Licensed under the "Elastic License 2.0".             #
################################################################################

try: 
    from cmk.gui.watolib.rulespecs import (
        rulespec_group_registry,
        RulespecSubGroup,
    )
    from cmk.gui.plugins.wato.utils import (
		HostRulespec,
  		rulespec_registry
	)
    from cmk.gui.valuespec import *
    from cmk.gui.i18n import _
    from cmk.gui.plugins.wato.special_agents.common import RulespecGroupDatasourceProgramsCustom
    from cmk.gui.plugins.wato.check_mk_configuration import RulespecGroupAgent
    
    def _create_custom_group(group):
        group_name = group.split("/")[1]
        @rulespec_group_registry.register
        class RulespecGroupSmartMAgentCustom(RulespecSubGroup):
            @property
            def main_group(self):
                return RulespecGroupAgent

            @property
            def sub_group_name(self):
                return "smartm_" + group_name.lower()

            @property
            def title(self):
                return _("SmartM " + group_name)
            
        return RulespecGroupSmartMAgentCustom
            

    def register_rule(group, name, vs, title = None, help = _, match = "dict"):
        group = RulespecGroupDatasourceProgramsCustom if group == "datasource_programs" \
            else _create_custom_group(group)
        rulespec_registry.register(
            HostRulespec(
                group = group,
                name = name,
                title = lambda: title or vs.title(),
                match_type = match,
                valuespec = lambda: vs
            )
        )
except: pass

def basic_auth(title = "Credentials", help = "Credentials used to log in",
               usertitle = "Username", passtitle = "Password",
               userhelp = "Username of the user", 
               passhelp = "password of the user"):
    return Dictionary(
        title = _(title),
        help = _(help),
        required_keys = ["username"],
        elements = [
            ("username", TextAscii(
                title = _(usertitle),
                help = _(userhelp)
            )),
            ("password", Password(
                title = _(passtitle),
                help = _(passhelp)
            ))
        ]
    )
    
def ntlm_auth(title = "Credentials", help = "Credentials used to log in"):
    return Dictionary(
        title = _(title),
        help = _(help),
        required_keys = ["username"],
        elements = [
            ("username", TextAscii(
                title = _("Username"),
                help = _("username of the user")
            )),
            ("domain", TextAscii(
                title = _("Domain"),
                help = _("domain of the user")
            )),
            ("password", Password(
                title = _("Password"),
                help = _("Password of the user")
            )),            
        ]
    )
    
def http_client(title = "Http Client", help = "Settings for the http client"):
    return Dictionary(
        title = _(title),
        help = _(help),
        required_keys = ["username"],
        elements = [
            ("timeout", Integer(
                title = _("Timeout"),
                help = _("Timeout per request send"),
                minvalue = 0,
                maxvalue = 90
            )),
            ("port", Integer(
                title = _("Port"),
                help = _("Port of the api"),
                minvalue = 1024,
                maxvalue = 2**16-1,
                default_value = 443
            )),
            ("host_allias", CascadingDropdown(
                title = _("Host Allias"),
                help = _("Name of the host we used in the http connection. "
                         "Use this if the name of the host in MonitorNow is not "
                         "the same as the CN of the https-certificate"),
                choices = [
                    ("Domain", "Domain", TextAscii(
                        help = _("Domain to be added to the original hostname"),
                    )),
                    ("Ip", "Ip", None),
                ]
            )),
            ("https_strategy", CascadingDropdown(
                title = _("HTTPS Strategy"),
                help = _("How to handle https when requesting data from the api.\n"
                         "Strict: Use system Certificates to verify the server certificate (default).\n"
                         "Specific: Use the provided certificate to verify the server certificate\n"
                         "Ignore Hostname: ignore hostname in the certificate, but still verify the content\n"
                         "Ignore Certificate: skip verification of the server certificate"
                         "HTTP: do not use https and go for http"),
                choices = [("strict", _("Strict"), None),
                           ("specific", _("Specific"), TextAscii()),
                           ("ignore_hostname", _("Ignore Hostname"), TextAscii()),
                           ("ignore_certificate", _("Ignore Certificate"), None),
                           ("http", _("Use HTTP"), None)]
            ))
        ]
    )

register_rule("datasource_programs",
    "special_agents:smartm",
    Dictionary(
        title = _("SmartM datasource options"),
        help = _("Options for the SmartM datasource"),
        elements = [
            #("run_legacy_datasources", Checkbox(
            #    title = _("Run legacy data sources"),
            #    label = _("Run legacy data sources"),
            #    help = _("Run additional datasources configured on the system, "
            #             "for compatibility with non-SmartM agents."))),
            ("write_smartm_data", Dictionary(
                title = _("Write SmartM data"),
                help = _("Write elastic data to be shipped to SmartM."),
                elements = [
                    ("instances", ListOfStrings(
                        title = _("Define custom instance list"),
                        help = _("Define a custom list of SmartM instances for which to write SmartM data.")))
                ])),
            ("use_password_vault", CascadingDropdown(
                title = _("Use password vault"),
                help = _("Use the password vault instead of credentials configured in WATO. "
                         "When this is enabled user names given in WATO are interpreted as "
                         "entries in the password vault."),
                choices = [
                    ("keepass", _("KeePass"))
                ])),
            ("error_reporting", Transform(
                CascadingDropdown(
                    title = _("Error reporting mechanism"),
                    help = _("Choose whether to handle error reporting in the smartm agent"
                             " (use when this is the only datasource or you don't care the"
                             " report might be late one scheduling cycle) or to integrate"
                             " with the legacy mechanism (use when multiple datasources are"
                             " configured and you want to make sure the dependency agent runs"
                             " last to avoid reporting delays)."),
                    choices = [
                        ("handle", _("Handle error reporting"), Dictionary(
                            title = _("Handle error reporting"),
                            elements = [("move_error_file", Transform(
                                FixedValue(
                                    '',
                                    title = _("Move error file after read"),
                                    help = _("Move the error file after reading it, to avoid"
                                             " reporting stale errors. This is useful when"
                                             " error reporting for non-inventorized checks is"
                                             " enabled (see the configuration for the Dependency"
                                             " Check).")),
                                back = lambda _: True,
                                forth = lambda _: '',
                            ))])),
                        ("legacy", _("Integrate with dependency agent (legacy)"))
                    ]),
                back = lambda v: {v[0]: v[1]} if isinstance(v, tuple) else v,
                forth = lambda v: next(iter(v.items())) if isinstance(v, dict) else v,
            )),
            ("run_noninventorized_checks", Transform(
                FixedValue(
                    '',
                    title = _("Run non-inventorized checks"),
                    help = _("Gather data for non-inventorized checks in monitoring mode."
                             " By default, queries are optimized by not including these checks."
                             " This option disables that optimization. It is advised to enable"
                             " this option when error reporting for non-inventorised checks is"
                             " enabled in the Dependency Check, to avoid hiding pre-requisite"
                             " errors for non-inventorised checks.")),
                back = lambda _: True,
                forth = lambda _: '',
            )),
            ("show_field_errors", Transform(
                FixedValue('',
                           title = _("Show field errors (debug)"),
                           help = _("Show field errors instead of no value ('-'). This is "
                                    "mainly useful for debugging purposes.")),
                back = lambda _: True,
                forth = lambda _: '',
            )),
            ("show_table_errors", Transform(
                FixedValue('',
                           title = _("Show table errors (debug)"),
                           help = _("Show table errors instead of no value ('-'). This is "
                                    "mainly useful for debugging purposes.")),
                back = lambda _: True,
                forth = lambda _: '',
            )),
        ],
        required_keys = [],
        ignored_keys = ['run_legacy_datasources', 'show_field_errors'],
        migrate = lambda v: [v.__setitem__('use_password_vault', 'keepass') if v.get('use_password_vault') is True
                             else v.__delitem__('use_password_vault') if v.get('use_password_vault') is False
                             else None,
                             v.__setitem__('write_smartm_data', {}) if v.get('write_smartm_data') is True
                             else  v.__delitem__('write_smartm_data') if v.get('write_smartm_data') is False
                             else None,
                             v][-1]
    ),
    title = _("SmartM Agent"),
    help = _("Use the SmartM agent."),
    match = 'dict')


# SNMP rules.

register_rule(
    "agent/SNMP",
    "snmp_optimization",
    Dictionary(
        title = _("SNMP bulk optimization settings (SmartM)"),
        help = _("Configure settings for SNMP bulk get/walk optimization."),
        required_keys = [],
        elements = [
            ("max_width", Integer(
                title = _("Maximum bulk request width"),
                help = _("The maximum number of OIDs to send in one bulk request. Some "
                         "devices refuse to respond requests over a certain size. "
                         "The default is 100."),
                minvalue = 1, default_value = 100)),
            ("max_length", Integer(
                title = _("Maximum walk request length"),
                help = _("The maximum number of parameters to walk in one bulk request. "
                         "Walks up to the maximum length are used for tables that are "
                         "known to be long."),
                minvalue = 1, default_value = 100)),
            ("max_size", Integer(
                title = _("Maximum bulk request size"),
                help = _("The maximum number of parameters to request in one bulk request "
                         "(gets + length x walks)."),
                minvalue = 1, default_value = 1000)),
            ("def_length", Integer(
                title = _("Default walk request length"),
                help = _("The number of parameters to walk in one bulk request when "
                         "the table length is unknown. This controls the compromise "
                         "between avoiding many small requests and requesting "
                         "unnecessary rows past the end of the table. "
                         "The default is 10. When latency is high, it may be useful "
                         "to increase this limit if the cost of making an additional "
                         "request outweighs the cost of requesting unnecessary data."),
                minvalue = 1, default_value = 10)),
            ("min_length", Integer(
                title = _("Minimal walk request length"),
                help = _("The number of parameters to walk in one bulk request when "
                         "the table length is longer than expected. This controls the "
                         "compromise between avoiding many small requests and requesting "
                         "unnecessary rows past the end of the table. "
                         "The default is 5. When latency is high, it may be useful "
                         "to increase this limit if the cost of making an additional "
                         "request outweighs the cost of requesting unnecessary data."),
                minvalue = 1, default_value = 10)),
            ("max_len_diff", Integer(
                title = _("Maximal walk request length difference"),
                help = _("The maximal number of useless rows to knowingly request in a "
                         "bulk request. Requests for useless rows happen when tables "
                         "of different expected length are combined in one request. "
                         "The default value is 5. When latency is high, it may be useful "
                         ""),
                minvalue = 1, default_value = 5)),

        ]
    )
)

register_rule(
    "agent/SNMP",
    "snmp_quirks",
    Dictionary(
        title = _("SNMP quirks (SmartM)"),
        help = _("Configure workarounds for the SNMP protocol on devices "
                 "that exhibit odd behaviour."),
        optional_keys = ["request_delay"],
        elements = [
            ("ignore_oids_not_increasing", Checkbox(
                title = _("Continue walk even when OIDs are not increasing"),
                label = _("Enabled")
            )),
            ("invalid_packets_at_end", Checkbox(
                title = _("Workaround for devices that respond "
                          "with an invalid packet if an oid past "
                          "the end of a table is requested"),
                label = _("Enabled")
            )),
            ("refresh_session", Checkbox(
                title = _("Refresh the netsnmp session on each request"),
                label = _("Enabled")
            )),
            ("request_delay", Integer(
                title = _("Number of milliseconds to wait between requests"),
                help = _("Number of milliseconds to wait between requests."),
                minvalue = 1,
                maxvalue = 500,
            ))
        ]
    )
)

# SSH rules.
register_rule(
    "agent/SSH",
    "ssh_authentication",
    Dictionary(
        title=_("Credentials"),
        required_keys = ["username"],
        elements=
        [			
            ("username", TextAscii(
                title = _("Username"),
                allow_empty = False
            )),
            
            (
                "credential_type",
                CascadingDropdown(
                    title=_("Method"),
                    choices=
                    [
                        (
                            "password",
                            _("Password"),
                            Dictionary(
                                title=_("Password"),
                                required_keys=["password"],
                                elements=[
                                    (
                                        "password",
                                        Password(
                                            title=_("Password"),
                                            help=_(
                                                "Authenticate using a password."
                                            ),
                                            allow_empty=True,
                                        ),
                                    )
                                ],
                            ),
                        ), # Einde dropdown choice
                        (
                            "identity_file",
                            _("Identity File"),
                            Dictionary(
                                help=_(
                                    "Authenticate using a public / private key-pair."
                                ),
                                required_keys=["identity_file"],
                                elements=[
                                    (
                                        "identity_file",
                                        TextAscii(
                                            title=_("Private key file"),
                                            help=_(
                                                "Private key file location."
                                            ),
                                            allow_empty=False,
                                        ),
                                    ),
                                    (
                                        "password",
                                        Password(
                                            title=_("Password"),
                                            help=_(
                                                "The password for the private key."
                                            ),
                                            allow_empty=True,
                                        ),
                                    ),
                                ],
                            ),
                        ),  # Einde dropdown choice
                    ]
                )
            ) # Einde dropdown
        ] # Einde credentiials elements
    ), # Einde credentials dictionary
)    

register_rule(
    "agent/SSH",
    "ssh_connectivity",
    Dictionary(
        title=_("Connectivity"),
        elements=[
            (
                "port",
                Integer(
                    title=_("Port"),
                    minvalue=1,
                    maxvalue=(2**16) - 1,
                    default_value=22,
                ),
            ),
            (
                "max_sessions",
                Integer(
                    title = _("Max Channels"),
                    help = _("Maximum number of ssh channels that can be opend in a single session"),
                    minvalue=1,
                    maxvalue=10,
                    default_value=10,
                )
            )
        ]
    ), # Einde Connectivity Dictionary
)

register_rule(
    "agent/SSH",
    "ssh_options",
    Dictionary(
        title=_("Options"),
        elements=[
            (
                "timeout",
                Integer(
                    title=_("Timeout"),
                    minvalue = 1,
                    maxvalue = 20,
                ),
            ),
            (
                "allow_sudo",
                Checkbox(title=_("Allow sudo")),
            ),
        ]
    ), # Einde Options Dictionary
)
            
register_rule(
    "agent/SSH",
    "ssh_jumphosts",
    ListOf(
        Dictionary(
            title = _("Jumphosts"),
            elements = [
                ("credentials", Dictionary(
                    title=_("Credentials"),
                    elements=
                    [			
                        ("username", TextAscii(
                            title = _("Username"),
                            allow_empty = False
                        )),
                        
                        (
                            "method",
                            CascadingDropdown(
                                title=_("Method"),
                                choices=
                                [
                                    (
                                        "password",
                                        _("Password"),
                                        Dictionary(
                                            title=_("Password"),
                                            required_keys=["password"],
                                            elements=[
                                                (
                                                    "password",
                                                    Password(
                                                        title=_("Password"),
                                                        help=_(
                                                            "Authenticate using a password."
                                                        ),
                                                        allow_empty=False,
                                                    ),
                                                )
                                            ],
                                        ),
                                    ), # Einde dropdown choice
                                    (
                                        "identity_file",
                                        _("Identity File"),
                                        Dictionary(
                                            help=_(
                                                "Authenticate using a public / private key-pair."
                                            ),
                                            required_keys=["identity_file"],
                                            elements=[
                                                (
                                                    "identity_file",
                                                    TextAscii(
                                                        title=_("Private key file"),
                                                        help=_(
                                                            "Private key file location."
                                                        ),
                                                        allow_empty=False,
                                                    ),
                                                ),
                                                (
                                                    "password",
                                                    Password(
                                                        title=_("Password"),
                                                        help=_(
                                                            "The password for the private key."
                                                        ),
                                                        allow_empty=True,
                                                    ),
                                                ),
                                            ],
                                        ),
                                    ),  # Einde dropdown choice
                                ]
                            )
                        ) # Einde dropdown
                    ] # Einde credentiials elements
                )), # Einde credentials dictionary
                


                ("connectivity", Dictionary(
                title=_("Connectivity"),
                elements=[
                    (
                        "port",
                        Integer(
                            title=_("Port"),
                            minvalue=1,
                            maxvalue=(2**16) - 1,
                            default_value=22,
                        ),
                    ),
                ]
            )), # Einde Connectivity Dictionary




            ]
        ), 
        title = _("Jumphosts"),
    )
)

# Azure rules
register_rule(
    "agent/Azure",
    "azure_tenant",
    Dictionary(
        title = _("Tenant"),
        help = _("Id of the client in azure"),
        required_keys = ["tenantId"],
        elements = [
            ("tenantId", TextAscii(
                title = _("Tenant Id"),
                help = _("Id of the client in azure"),
                allow_empty = False
            )),
        ]
    )
)

register_rule(
    "agent/Azure",
    "azure_client",
    Dictionary(
        title = _("Client"),
        help = _("User in azure"),
        required_keys = ["clientId"],
        elements = [
            ("clientId", TextAscii(
                title = _("Client Id"),
                help = _("Id of the user logging in"),
                allow_empty = False
            )),
            ("clientSecret", Password(
                title = _("Client Secret"),
                help = _("Password / secret of the user logging in"),
                allow_empty = False
            )), ("ClientName", TextAscii(
                title = _("Name of the client"),
                help = _("Name of the client in the keyvault"),
            ))
        ]
    )
)

register_rule(
    "agent/Azure",
    "azure_subscriptions",
    Dictionary(
        title = _("Subscriptions"),
        help = _("Subscriptions to monitor"),
        required_keys = ["subscriptions"],
        elements = [
            ("subscriptions", ListOfStrings(
                title = _("Subscriptions"),
                help = _("Subscriptions to monitor")
            ))
        ]
    )
)

register_rule(
    "agent/Azure",
    "azure_resourceGroups",
    Dictionary(
        title = _("resourceGroups"),
        help = _("ResourceGroups to monitor. By default all resourcegroups are monitored."),
        required_keys = ["resourceGroups"],
        elements = [
            ("resourceGroups", TextAscii(
                title = _("resourceGroups"),
                help = _("Regex of resourceGroups to monitor. By default all resourcegroups are monitored.")
            ))
        ]
    )
)

# WinRM
register_rule(
    "agent/WinRM",
    "winrm_credentials",
    Alternative(
        title = _("Credentials"),
        help = _("Credentials for WinRM"),
        elements = [
            Tuple(
                title = _("Basic"),
                orientation = "horizontal",
                elements = [
                    FixedValue("Basic"),
                    Dictionary(
                        # title = _("Basic Authentication"),
                        help = _("Use basic authentication to connect to the winrm service"),
                        elements = [
                            ("username", TextAscii(
                                title = _("Username"),
                                help = _("Username of the user.")
                            )), ("password", Password(
                                title = _("Password"),
                                help = _("Password of the user.")
                            ))
                        ],
                        required_keys = ["username"],
                    )
                ]
            ),
            Tuple(
                title = _("Ntlm"),
                orientation = "horizontal",
                elements = [
                    FixedValue("Ntlm"),
                    Dictionary(
                        # title = _("NTML Authentication"),
                        help = _("Use ntml authentication to connecto the winrm service."),
                        elements = [
                            ("username", TextAscii(
                                title = _("Username"),
                                help = _("Username of the user.")
                            )), ("password", Password(
                                title = _("Password"),
                                help = _("Password of the user.")
                            )), ("domain", TextAscii(
                                title = _("Domain"),
                                help = _("Domain of the user")
                            ))
                        ],
                        optional_keys = ["password", "domain"]
                    )
                ]
            ),
            Tuple(
                title = _("Certificate"),
                orientation = "horizontal",
                elements = [
                    FixedValue("Certificate"),
                    Tuple(
                        help = "The public and private parts of the certificate used to log in to the server (PEM format)",
                        elements = [
                            TextAscii(
                                title = "Private Key", 
                                help = "private key of the client certificate, used to log in"
                            ),
                            TextAscii(
                                title = "Certificate", 
                                help = "public part of the client certificate, used to log in"
                            )
                        ]
                    )
                ]
               ),
               Tuple(
                title = _("Kerberos"),
                orientation = "horizontal",
                elements = [
                    FixedValue("Kerberos"),
                    Dictionary(
                        help = _("Use kerberos authentication to connecto the winrm service. "
                            "NOTE: kinit must be scheduled to retrieve the TGT seperatly"),
                        required_keys = ["realm"],
                        elements = [
                            ("realm", TextAscii(
                                title = _("Realm"),
                                help = _("Realm to log in to")
                            )),
                            ("ccache_name", Alternative(
                                title = _("ccache_name"),
                                help = _("Name of the ccache for the tickets of the realm. "
                                    "This value only overwrites the system default, which is defined in /etc/krb5.conf "
                                    "This can be a DIR, FILE KCM or KEYRING. For more info, see:"
                                    "http://web.mit.edu/kerberos/krb5-devel/doc/basic/ccache_def.html. "),
                                elements = [
                                    Tuple(
                                        title = _("DIR"),
                                        help = _("points to the storage location of the collection of the credential caches in FILE: format. "
                                            "It is most useful when dealing with multiple Kerberos realms and KDCs"),
                                        orientation = "horizontal",
                                        elements = [FixedValue("DIR"), TextAscii()]
                                    ),
                                    Tuple(
                                        title = _("FILE"),
                                        help = _("caches are the simplest and most portable. A simple flat file format is used to store one credential after another"),
                                        orientation = "horizontal",
                                        elements = [FixedValue("FILE"), TextAscii()]
                                    ),
                                    Tuple(
                                        title = _("KCM"),
                                        help = _("caches work by contacting a daemon process called kcm to perform cache operations"),
                                        orientation = "horizontal",
                                        elements = [FixedValue("KCM "), TextAscii()]
                                    ),
                                    Tuple(
                                        title = _("KEYRING"),
                                        help = _("is Linux-specific, and uses the kernel keyring support to store credential data in "
                                            " unswappable kernel memory where only the current user should be able to access it"),
                                        orientation = "horizontal",
                                        elements = [FixedValue("KEYRING"), TextAscii()]
                                    ),
                                ]
                            ))
                        ]
                    )
                ]
            )
        ]
    )
)

register_rule(
    "agent/WinRM",
    "winrm_options",
    Dictionary(
        title = _("Options"),
          help = _("Options for WinMR shells and commands"),
        required_keys = ["noprofile", "skip_cmd_shell"],
        elements = [
            ("noprofile", Checkbox(
                title = _("No Profile"),
                help = _("If set to TRUE, this option specifies that the user profile does "
                         "not exist on the remote system and that the default profile SHOULD be "
                        "used. By default, the value is TRUE."),
                default_value = True				
            )),
            ("skip_cmd_shell", Checkbox(
                title = _("Skip Cmd Shell"),
                help = _("If set to TRUE, this option requests that the server runs the command "
                         "without using cmd.exe; if set to FALSE, the server is requested to use cmd.exe. "
                        "By default the value is FALSE. This does not have any impact on the wire protocol."),
                default_value = False
            ))
        ]
    )
)

winrm_connectivity = Dictionary(
    title = _("Connectivity"),
    help = _("Connection options for WinRM"),
    required_keys = ["https", "disable_certificate_verification", 
                       "disable_hostname_verification", "built_in_root_certs"],
    elements = [
        ("https", Checkbox(
            title = _("Use https"),
            help = _("Check the box to use https. Https is enabled by default")
        )), ("port", Integer(
            title = _("Port"),
            help = _("Port of the WinRM Service"),
            minvalue = 1,
            maxvalue = 65535,
        )), ("timeout", Integer(
            title = _("Timeout"),
            help = _("Timeout of per command, in seconds"),
            minvalue = 1,
            maxvalue = 90,
        )), ("certificate", Tuple(
            title = _("Certificate"),
            elements = [
                DropdownChoice(
                    title = _("Certificate file type"),
                    choices = [
                        ("PEM", "PEM"), 
                        ("DER", "DER")
                    ]
                ),
                TextAscii(
                    title = _("Location"),
                    help = _("Location of the certificate chain on the fileserver")
                )
            ]
        )), ("host_allias", CascadingDropdown(
            title = _("Host Allias"),
            help = _("Name of the host we used in the http connection. Use this if the name of the host in MonitorNow is not the same as the CN of the https-certificate"),
            choices = [
                ("Domain", "Domain", TextAscii(
                    help = _("Domain to be added to the original hostname"),
                )),
                ("Ip", "Ip", None),
            ]
        )),
        ("disable_certificate_verification", Checkbox(
            title = _("Disable Certificate Verification (DANGER)"),
            help = _("Disables certificate verificaton. Active this at your own risk. "
                    "You should think very carefully before using this method. "
                    "If invalid certificates are trusted, any certificate for any site will be trusted for use. "
                    "This includes expired certificates. "
                    "This introduces significant vulnerabilities, and should only be used as a last resort."),
        )),
        ("disable_hostname_verification", Checkbox(
            title = _("Disable Hostname Verification (DANGER)"),
            help = _("Disables hostname verification on the certificate. Active this at your own risk. "
                    "You should think very carefully before you use this method. "
                    "If hostname verification is not used, any valid certificate for any site will be trusted for use from any other. "
                    "This introduces a significant vulnerability to man-in-the-middle attacks"),
        )),
        ("built_in_root_certs", Checkbox(
            title = _("Use System Sertificates"),
            help = _("Check the box to only allow default/external certificates, if false use our own certificate")
        )),
    ]
)

winagent_connectivity = Dictionary(
    title = _("Windows Agent"),
    help = _("Connection options for the Windows Agent"),
    elements = [
        ("port", Integer(
            title = _("Port"),
            help = _("Port of the Windows Agent"),
            minvalue = 1024,
            maxvalue = 65535,)),
        ("server_root_cert", TextAscii(
            title = _("Server Root Cert"))),
        ("connection_timeout", Integer(
            title = _("Timeout"),
            help = _("Timeout of per command, in seconds"),
            minvalue = 1,
            maxvalue = 90,)),
    ]
)

def into_2_20(vs):
    if isinstance(vs, dict): # old winrm config
        opts = {"https": True,
                "built_in_root_certs": True,
                "disable_certificate_verification": False,
                "disable_hostname_verification": False}
        for opt, state in opts.items():
            vs[opt] = vs.get(opt, state)
        vs = ("WinRM", vs)
    return vs

def into_2_19(vs):
    return vs[1] if isinstance(vs, tuple) else vs

register_rule(
    "agent/WinRM",
    "winrm_connectivity",
    Transform(
        CascadingDropdown(
            title = _("Connectivity"),
               help  =_("Connectivity options to connect to windows servers"),
            choices = [
                ("WinRM", "WinRM", winrm_connectivity),
                ("WindowsAgent", "Windows Agent", winagent_connectivity)
            ]
        ),
        # back = into_2_19,
        forth = into_2_20
    )
)

register_rule(
    "agent/WMI",
    "wmic_connectivity",
    Dictionary(
        title = _("DCOM"),
        required_keys = ["credentials", "use_sudo"],
        elements = [
            ("credentials", CascadingDropdown(
                title = _("Credentials"),
                help = _("Credentials used to connect with DCOM"),
                choices = [
                    ("passwd_file", _("Password File"), TextAscii(
                        title =  _("Password File"),
                        help = _("location of the passwword file. contains 3 entries: "
                                 "username, password and domain (optional)")
                    )),
                    ("ntlm", _("Ntml"), Dictionary(
                        title =  _("Ntlm"),
                        required_keys = ["username"],
                        elements = [
                            ("username", TextAscii(title = _("Username"))),
                            ("password", Password(title = _("Password"))),
                            ("domain", TextAscii(title = _("Domain")))                            
                        ]
                    ))
                ]
            )),
            ("timeout", Integer(
                title = _("Timeout"),
                help = _("Timeout per wmic command"),
                default_value = 10
            )),
            ("use_sudo", Checkbox(
                title = _("Use sudo"),
                help = _("Use sudo to get the credentials from the passwordfile, (if applicable)"
                         "NOTE that the siteuser must be allowed to use passwordless sudo on the command '/usr/bin/cat <pwdfile>'"),
                default_value = False
            ))
        ]
    )
)

register_rule(
    "agent/WinRM",
    "powershell_context",
    ListOf(
        Tuple(
            elements = [
            TextInput(title = _("Key")),
                CascadingDropdown(
                    title = _("Value"),
                    choices = [
                        ("Text", "Text", TextAscii()),
                        ("Password", "Password", Password()),
                    ]
                )
            ]
        ),
        title = _("Powershell Context"),
        help = _("Powershell uses templates of scripts to fill in parameters such as credentials or context specific variables")
    )
)

# WMI
register_rule(
    "agent/WMI",
    "wmi_options",
    Dictionary(
        title = _("Options"),
        required_keys = ["wmi_method"],
        elements = [
            ("wmi_method", DropdownChoice(
                title = _("Wmi Method"),
                help = _("How do we obtain the wmi classes. Using Get-WmiObject or Get-CimInstance. "
                         "GetCimInstance is more robust and modern. However, it is not supported on all Windows Servers. "
                         "GetWmiObject is available on all Windows Servers with powershell. But can crash on, for example, random tabs in service names. "
                         "EnumerateCimInstance uses the winrm buildin enumeration method to retrieve the ciminstances. "
                         "GetCimInstance is used by default. If you work on older servers, and Get-CimInstance is not available, use Get-WmiObject."),
                choices = [
                    ("GetWmiObject", "GetWmiObject"), 
                    ("GetCimInstance", "GetCimInstance"),
                    ("EnumerateCimInstance", "EnumerateCimInstance"),
                ]
            ))			
        ]
    )
)

register_rule(
    "agent/WMI",
    "wmi_quircks",
    Dictionary(
        title = _("Wmi Quircks"),
        elements = [
            ("local_as_utc", Dictionary(
                title = _("Local as Utc"),
                help = _("There are fields that return localtime without a timezone. as a result, they are interpreted as UTC. "
                    "Here you can define the class and field that is victim to this bug, and the timezone of the server."),
                required_keys = ["timezone"],
                elements = [
                    ("timezone", TextAscii(title = _("Timezone"))),
                    ("fields", ListOf(Tuple(elements = [
                        TextAscii(title = _("Class")),
                        TextAscii(title = _("Field"))
                    ])))
                ]
            ))
        ]
    )
)

register_rule(
    "agent/WMI",
    "wmi_retries",
    Integer(
        title = _("Retries"),
        help = _("How often will we retry the wmi commands if we recieve an error. This is 0 default")
    )
)

# API plugins
# Vmware
register_rule(
    "agent/Vmware",
    "vmware_connectivity",
    Dictionary(
        title = _("Connectivity"),
        help = _("Connection settings used to connect to the vmware SDK"),
        elements = [
            ("port", Integer(
                title = _("Port"),
                help = _("Port of the vmware web service."),
                minvalue = 1,
                maxvalue = 65535
            )),
            ("certificate", Tuple(
                title = _("Certificate"),
                elements = [
                    DropdownChoice(
                        title = _("Certificate file type"),
                        choices = [
                            ("PEM", "PEM"), 
                            ("DER", "DER")
                        ]
                    ),
                    TextAscii(
                        title = _("Location"),
                        help = _("Location of the certificate chain on the fileserver")
                    )
                ]
            )),
            ("host_allias", CascadingDropdown(
                title = _("Host Allias"),
                help = _("Name of the host we used in the http connection. Use this if the name of the host in MonitorNow is not the same as the CN of the https-certificate"),
                choices = [
                    ("Domain", "Domain", TextAscii(
                        help = _("Domain to be added to the original hostname"),
                    )),
                    ("Ip", "Ip", None),
                ]
            )),
            ("disable_certificate_verification", Checkbox(
                title = _("Disable Certificate Verification (DANGER)"),
                help = _("Disables certificate verificaton. Active this at your own risk. "
                        "You should think very carefully before using this method. "
                        "If invalid certificates are trusted, any certificate for any site will be trusted for use. "
                        "This includes expired certificates. "
                        "This introduces significant vulnerabilities, and should only be used as a last resort."),
            )),
            ("disable_hostname_verification", Checkbox(
                title = _("Disable Hostname Verification (DANGER)"),
                help = _("Disables hostname verification on the certificate. Active this at your own risk. "
                        "You should think very carefully before you use this method. "
                        "If hostname verification is not used, any valid certificate for any site will be trusted for use from any other. "
                        "This introduces a significant vulnerability to man-in-the-middle attacks"),
            )),
        ]
    )
)

register_rule(
    "agent/Vmware",
    "vmware_is_cluster",
    Dictionary(
        title = _("Cluster"),
        help = _("Whether this host represesnts the vmware clsuter or not"),
        required_keys = ["is_cluster"],
        elements = [
            ("is_cluster", Checkbox(
                title = _("Cluster"),
                help = _("Whether this host represesnts the vmware clsuter or not."),
            ))
        ]
    )
)

register_rule(
    "agent/Vmware",
    "vmware_credentials",
    Dictionary(
        title = _("Credentials"),
        help = _("Credentials for the vSphere Web Services SDK"),
        elements = [
            ("username", TextAscii(
                title = _("Username"),
                help = _("Username of the user.")
            )), ("password", Password(
                title = _("Password"),
                help = _("Password of the user.")
            ))
        ]
    )
)

# Ms Graph
register_rule(
    "agent/MSGraph",
    "ms_graph_tenant",
    Dictionary(
        title = _("Tenant"),
        help = _("Id of the tenant"),
        required_keys = ["tenantId"],
        elements = [
            ("tenantId", TextAscii(
                title = _("Tenant Id"),
                help = _("Id of the client in azure"),
                allow_empty = False
            )),
        ]
    )
)

register_rule(
    "agent/MSGraph",
    "ms_graph_client",
    Dictionary(
        title = _("Client"),
        help = _("User for MS Graph"),
        required_keys = ["clientId"],
        elements = [
            ("clientId", TextAscii(
                title = _("Client Id"),
                help = _("Id of the user logging in"),
                allow_empty = False
            )),
            ("clientSecret", Password(
                title = _("Client Secret"),
                help = _("Password / secret of the user logging in"),
            )), ("clientName", TextAscii(
                title = _("Name of the client"),
                help = _("Name of the client in the keyvault"),
            ))
        ]
    )
)

register_rule(
    "agent/SQL",
    "sql_instances",
    Dictionary(
        title = _("Instances"),
          help = _("Instances available on the server. These are port numbers, or in the case of MSSQL, they can be the names of the instance. If no instance is provided, the default of the driver is used"),
        required_keys = ["instances"],
          elements = [
            ("instances", ListOfStrings())
        ]
    )
)

register_rule(
    "agent/SQL",
    "sql_credentials",
    Dictionary(
        title = _("Credentials"),
        help = _("Credentials used to log in to the instances on the host"),
        required_keys = ["username"],
        elements = [
            ("username", TextAscii(title = _("Username"), help = _("Name of the user, or location in the keyvault"))),
            ("password", Password(title= _("Password"), help = _("Password of the user. This can be left empty when using the keyvault")))
        ]
    )
)

register_rule(
    "agent/SQL",
    "sql_odbc_options",
     Dictionary(
        title = ("ODBC Options"),
        help = ("Options to be provided to the odbc driver. Most of these options can be defined in a DSN"),
        elements = [
            ("driver", TextAscii(
                title = _("Driver"),
                help = _("ODBC Driver to be used. These must be installed on the system for us to connect to the database. For information on how to install these drivers; visit: https://github.com/mkleehammer/pyodbc/wiki/Connecting-to-databases"),
            )),
            ("dsn", TextAscii(
                title = _("DSN"),
                help = _("Which dsn should be used. A DSN is a template that is used to build a connection string. This can be conigured user- or systemwide")
            )),
            ("file_dsn", TextAscii(
                title = _("File DSN"),
                help = _("Similar to DSN, but from a specific file.")
            )),
            ("database", TextAscii(
                title = _("Database"),
                help = _("The inital database we should connect to")
            )),
               ("timeout", Integer(
                title = _("Timeout"),
                help = _("The timeout used to create the inital connection and per query. WARNING: Do not use this with postgresql. The driver crashes when useing this argument....")
            )),
            ("ssl", DropdownChoice(
                title = _("SSL Preference"),
                help = _("Our stance on whether we should use SSL or not."),
                choices = [(val, val) for val in ["Require", "Prefer", "Allow", "Disable"]]
            )),
            ("ssl_key", TextAscii(
                title = _("SSL Key"),
                help = _("The absolute path to the client\'s private key file")
            )),
            ("ssl_cert", TextAscii(
                title = _("SSL Certificate"),
                help = _("The absolute path of the client\'s public certificate file")
            )),
            ("disable_certificate_verification", Checkbox(
                title = _("Trust Server Certificate"),
                help = _("Trust the server certificate and skip verification")
            )),
            ("encrypt", Checkbox(
                title = _("Force Encryption"),
                help = _("force the use of encryption using the ssl certificate of the server")
            )),
               ("connection_string", TextAscii(
                  title = _("Connection String"),
                help = _("The raw connection string that should be used for every instance. Used for debugging purposses")
            )),
            ("custom_args", ListOf(
                Tuple(elements = [TextAscii(), TextAscii()]),
                title = _("Custom Arguments"),
                help = ("custom Key / Value pairs used to generate the connectionstring")
            ))
        ]
    )
)

register_rule(
    "agent/MSGraph",
    "ms_graph_rapports",
    Dictionary(
        title = _("Rapports"),
        help = _("Filter and Sorting configuration for ms graph rapports"),
        elements = [
            ("onedrive_usage", Tuple(
                title = _("Onedrive Usage"),
                help = _("How to sort the the rapports and give the given top"),
                elements = [
                    Integer(
                        title = _("TOP"),
                        help = _("The maximun number of rows to be displayed in the check"),
                        minvalue = 1, default_value = 200
                    ),
                    DropdownChoice(
                        title = _("Sort Key"),
                        help = _("The key used to sort the rows in the table"),
                        choices = [
                            ("Owner", "Owner"),
                            ("SiteURL", "SiteURL"),
                            ("FileCount", "FileCount"),
                            ("ActiveFileCount", "ActiveFileCount"),
                            ("StorageUsed", "StorageUsed"),
                            ("StorageUsedRel", "StorageUsedRel"),
                            ("LastActivity", "LastActivity"),
                        ]
                    )
                ]
            )),
            ("outlook_usage", Tuple(
                title = _("Outlook Usage"),
                help = _("How to sort the the rapports and give the given top"),
                elements = [
                    Integer(
                        title = _("TOP"),
                        help = _("The maximun number of rows to be displayed in the check"),
                        minvalue = 1, default_value = 200
                    ),
                    DropdownChoice(
                        title = _("Sort Key"),
                        help = _("The key used to sort the rows in the table"),
                        choices = [
                            ("UserPrincipalName", "UserPrincipalName"),
                            ("ItemCount", "ItemCount"),
                            ("StorageUsed", "StorageUsed"),
                            ("StorageUsedRel", "StorageUsedRel"),
                            ("DeletedItemSize", "DeletedItemSize"),
                            ("DeletedItemSizeRel", "DeletedItemSizeRel"),
                            ("LastActivity", "LastActivity"),
                        ]
                    )
                ]
            )),
            ("sharepoint_usage", Tuple(
                title = _("Sharepoint Usage"),
                help = _("How to sort the the rapports and give the given top"),
                elements = [
                    Integer(
                        title = _("TOP"),
                        help = _("The maximun number of rows to be displayed in the check"),
                        minvalue = 1, default_value = 200
                    ),
                    DropdownChoice(
                        title = _("Sort Key"),
                        help = _("The key used to sort the rows in the table"),
                        choices = [
                            ("Owner", "Owner"),
                            ("SiteURL", "SiteURL"),
                            ("FileCount", "FileCount"),
                            ("ActiveFileCount", "ActiveFileCount"),
                            ("PageViews", "PageViews"),
                            ("VisitedPages", "VisitedPages"),
                            ("StorageUsed", "StorageUsed"),
                            ("StorageUsedRel", "StorageUsedRel"),
                            ("LastActivity", "LastActivity"),
                        ]
                    )
                ]
            ))
        ]
    )
)

register_rule(
    "agent/Ldap",
    "ldap",
    ListOf(
        Dictionary(
            title = _("Ldap"),
            help = _("Ldap config"),
            required_keys = ["host_config", "search_config"],
            elements = [
                ("service_name", TextAscii(
                    title = _("Service Name"),
                    help = _("Name of the running ldap service"),
                )),
                ("host_config", Dictionary(
                    title = _("Host Configuration"),
                    help = _("Configuration for host specific parameters"),
                    required_keys = ["ssl", "danger_disable_tls_verification", "danger_disable_hostname_verification"],
                    elements = [
                        ("timeout", Integer(
                            title = _("Timeout"),
                            help = _("Timeout in seconds for every request (bind, search, ...). Defaults to 10."),
                            minvalue = 1, default_value = 10,
                        )),
                        ("ssl", Checkbox(
                            title = _("Use SSL"),
                            help = _("Use ssl to encrypt the ldap connection (aka ldaps)"),
                        )),
                        ("port", Integer(
                            title = _("Port"),
                            help = _("Port used to connect to the ldap service"),
                            minvalue = 1, maxvalue = 65535,	default_value = 389,
                        )),
                        ("certificate", Tuple(
                            title = _("Certificate"),
                            elements = [
                                DropdownChoice(
                                    title = _("Certificate file type"),
                                    choices = [
                                        ("Pem", "PEM"), 
                                        ("Der", "DER")
                                    ]
                                ),
                                TextAscii(
                                    title = _("Location"),
                                    help = _("Location of the certificate chain on the fileserver")
                                )
                            ]
                        )),
                        ("danger_disable_tls_verification", Checkbox(
                            title = _("Disable TLS Verification (DANGER)"),
                            help = _("Disables certificate verificaton. Active this at your own risk. "
                                    "You should think very carefully before using this method. "
                                    "If invalid certificates are trusted, any certificate for any site will be trusted for use. "
                                    "This includes expired certificates. "
                                    "This introduces significant vulnerabilities, and should only be used as a last resort."),
                            default_value = False,
                        )),
                        ("danger_disable_hostname_verification", Checkbox(
                            title = _("Disable Hostname Verification (DANGER)"),
                            help = _("Disables hostname verification on the certificate. Active this at your own risk. "
                                    "You should think very carefully before you use this method. "
                                    "If hostname verification is not used, any valid certificate for any site will be trusted for use from any other. "
                                    "This introduces a significant vulnerability to man-in-the-middle attacks"),
                            default_value = False,
                        )),
                    ]
                )),
                ("bind_config", Dictionary(
                    title = _("Bind Configuration"),
                    help = _("Configuration used for binding to the service (authentication)"),
                    required_keys = ["bind_user"],
                    elements = [
                        ("bind_user", TextAscii(
                            title = _("Bind User"),
                            help = _("Username of the user"),
                        )),
                        ("bind_pass", Password(
                            title = _("Bind Password"),
                            help = _("Password of the user"),
                        ))
                    ]
                )),
                ("replication_config", CascadingDropdown(
                    title = _("Replication Configuration"),
                    help = _("What replication status should be checked?"),
                    choices = [
                        ("None", _("None"), None),
                        ("All", _("All"), None),
                        ("Specific", _("Specific"), ListOfStrings(
                            title = _("Suffix"),
                            help = _("Name of the suffix to monitor"),
                        )),
                    ]
                )),
                ("search_config", ListOf(
                    Dictionary(
                        title = _("Search Configuration"),
                        help = _("Configuration containing search queries"),
                        required_keys = ["base_dn", "scope"],
                        elements = [
                            ("base_dn", TextAscii(
                                title = _("Base DN"),
                                help = _("The point from where the search starts"),
                            )),
                            ("scope", DropdownChoice(
                                title = _("Scope"),
                                help = _("The scope of the search. "
                                    "Base: Base object; search only the object named in the base DN. "
                                    "OneLevel: Search the objects immediately below the base DN "
                                    "Subtree: Search the object named in the base DN and the whole subtree below it"),
                                choices = [
                                    ("Base", "Base"),
                                    ("OneLevel", "OneLevel"),
                                    ("Subtree", "Subtree")
                                ],
                            )),
                            ("filter", TextAscii(
                                title = _("Filter"),
                                help = _("An ldap filter to be applied to the search"),
                            )),
                            ("attributes", ListOfStrings(
                                title = _("Attributes"),
                                help = _("Attributes to be reauested from the found items. "
                                    "If attributes is empty, or if it contains a special name * (asterisk), return all (user) attributes. "
                                    "Requesting a special name + (plus sign) will return all operational attributes."
                                    "Include both * and + in order to return all attributes of an entry."),
                            ))
                        ]
                    ),
                    title = _("Search Configuration"),
                    help = _("Configuration containing search queries"),
                ))
            ]
        ), 
        title = _("Ldap"),
        help = _("Ldap config")
    )
)

register_rule(
    "agent/cache",
    "cache_connectivity",
    Dictionary(
        title = _("Connectivity"),
        help = _("Connection settings used to connect to the vmware SDK"),
        elements = [
            ("port", Integer(
                title = _("Port"),
                help = _("Port of the vmware web service."),
                minvalue = 1,
                maxvalue = 65535
            )),
            ("timeout", Integer(
                title = _("Timeout"),
                help = _("Timeout per request, in seconds"),
                minvalue = 1,
                maxvalue = 60,
                default_value = 10
            )),
            ("certificate", Tuple(
                title = _("Certificate"),
                elements = [
                    DropdownChoice(
                        title = _("Certificate file type"),
                        choices = [
                            ("PEM", "PEM"), 
                            ("DER", "DER")
                        ]
                    ),
                    TextAscii(
                        title = _("Location"),
                        help = _("Location of the certificate chain on the fileserver")
                    )
                ]
            )),
            ("host_allias", CascadingDropdown(
                title = _("Host Allias"),
                help = _("Name of the host we used in the http connection. Use this if the name of the host in MonitorNow is not the same as the CN of the https-certificate"),
                choices = [
                    ("Domain", "Domain", TextAscii(
                        help = _("Domain to be added to the original hostname"),
                    )),
                    ("Ip", "Ip", None),
                ]
            )),
            ("disable_certificate_verification", Checkbox(
                title = _("Disable Certificate Verification (DANGER)"),
                help = _("Disables certificate verificaton. Active this at your own risk. "
                        "You should think very carefully before using this method. "
                        "If invalid certificates are trusted, any certificate for any site will be trusted for use. "
                        "This includes expired certificates. "
                        "This introduces significant vulnerabilities, and should only be used as a last resort."),
            )),
            ("disable_hostname_verification", Checkbox(
                title = _("Disable Hostname Verification (DANGER)"),
                help = _("Disables hostname verification on the certificate. Active this at your own risk. "
                        "You should think very carefully before you use this method. "
                        "If hostname verification is not used, any valid certificate for any site will be trusted for use from any other. "
                        "This introduces a significant vulnerability to man-in-the-middle attacks"),
            )),
        ]
    )
)

register_rule(
    "agent/cache",
    "cache_credentials",
    Dictionary(
        title = _("Credentials"),
        help = _("Credentials for the vSphere Web Services SDK"),
        elements = [
            ("username", TextAscii(
                title = _("Username"),
                help = _("Username of the user.")
            )), ("password", Password(
                title = _("Password"),
                help = _("Password of the user.")
            ))
        ]
    )
)

register_rule(
    "agent/Mirth",
    "mirth_api_auth",
    basic_auth("API Credentials")
)

register_rule(
    "agent/Mirth",
    "mirth_http_client",
    http_client()
)

register_rule(
    "agent/Mirth",
    "mirth_smb_auth",
    ntlm_auth("SMB Credentials")
)

register_rule(
    "agent/Mirth",
    "mirth_smb_opts",
    Dictionary(
        title = _("SMB Options"),
        help = _("Options for SMB filechecking"),
        required_keys = ["username"],
        elements = [            
            ("max_concurrent", Integer(
                title = _("Max Concurrent"),
                help = _("How many smb clients can be started at once to execute smb requests"),
                default_value = 20
            )),
            ("server_mapping", ListOf(
                Tuple(elements=[TextAscii(), TextAscii()]),
                title = _("Server Mapping"),
                help = _("Mapping of mirth servers to smb servers")
            ))
        ]
    )
)

register_rule(
    "agent/Dell Unity",
    "dell_unity_creds",
    basic_auth()
)

register_rule(
    "agent/Dell Unity",
    "dell_unity_http",
    http_client()
)

register_rule(
    "agent/Citrix Xenapp Director",
    "xenapp_director_auth",
    ntlm_auth()
)

register_rule(
    "agent/Citrix Xenapp Director",
    "xenapp_director_client",
    http_client()
)

register_rule(
    "agent/Citrix Xenapp Director",
    "xenapp_director_server",
    TextAscii(
        title = _("Director Server"),
        help = _("If the host being requested is not the director server "
                 "than the host being requested will be the sever provided in this rule."
                 "The query will also filter the on host being polled. "
                 "In essence we are emulating the piggyback system")
    )
)

register_rule(
    "agent/Proxmox VE",
    "proxmox_creds",
    # basic_auth(usertitle = "Token ID",
    #            passtitle = "Secret")
    basic_auth(userhelp="username with the format <username>@<realm>")
)

register_rule(
    "agent/Proxmox VE",
    "proxmox_http",
    http_client()
)

register_rule(
    "agent/ElasticSearch",
    "elastic_http",
    http_client()
)

register_rule(
    "agent/ElasticSearch",
    "elastic_auth",
    basic_auth()
)
