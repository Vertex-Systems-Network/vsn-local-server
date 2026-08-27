Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Convert-Pkg0313StringArray {
  param([object]$Value)
  if ($null -eq $Value) { return @() }
  return @($Value | ForEach-Object { [string]$_ } | Sort-Object -Unique)
}

function Get-Pkg0313OptionalProperty {
  param([object]$Object,[string]$Name)
  if ($null -eq $Object) { return $null }
  $property = $Object.PSObject.Properties[$Name]
  if ($null -eq $property) { return $null }
  return $property.Value
}

function Convert-Pkg0313FirewallFilter {
  param([object]$Filter,[string[]]$Properties)
  $row = [ordered]@{}
  foreach ($property in $Properties) {
    # Firewall filter CIM shapes differ by Windows image/version. A property
    # absent from the object's schema is normalized to null instead of becoming
    # a StrictMode failure. A real value change still changes the snapshot.
    $value = Get-Pkg0313OptionalProperty $Filter $property
    if ($value -is [Array]) {
      $row[$property] = @(Convert-Pkg0313StringArray $value)
    } elseif ($null -eq $value) {
      $row[$property] = $null
    } else {
      $row[$property] = [string]$value
    }
  }
  return [pscustomobject]$row
}

function Get-Pkg0313FirewallSnapshot {
  $profiles = @(
    Get-NetFirewallProfile -PolicyStore PersistentStore -ErrorAction Stop |
      Sort-Object Name |
      ForEach-Object {
        [pscustomobject][ordered]@{
          name = [string]$_.Name
          enabled = [string]$_.Enabled
          default_inbound_action = [string]$_.DefaultInboundAction
          default_outbound_action = [string]$_.DefaultOutboundAction
          allow_inbound_rules = [string]$_.AllowInboundRules
          allow_local_firewall_rules = [string]$_.AllowLocalFirewallRules
          allow_local_ipsec_rules = [string]$_.AllowLocalIPsecRules
          allow_user_apps = [string]$_.AllowUserApps
          allow_user_ports = [string]$_.AllowUserPorts
          allow_unicast_response_to_multicast = [string]$_.AllowUnicastResponseToMulticast
          notify_on_listen = [string]$_.NotifyOnListen
          enable_stealth_mode_for_ipsec = [string]$_.EnableStealthModeForIPsec
          log_file_name = [string]$_.LogFileName
          log_max_size_kilobytes = [string]$_.LogMaxSizeKilobytes
          log_allowed = [string]$_.LogAllowed
          log_blocked = [string]$_.LogBlocked
        }
      }
  )

  $rules = @(Get-NetFirewallRule -PolicyStore PersistentStore -ErrorAction Stop | Sort-Object Name)
  $ruleRows = @(
    $rules | ForEach-Object {
      [pscustomobject][ordered]@{
        name = [string]$_.Name
        display_name = [string]$_.DisplayName
        description = [string]$_.Description
        display_group = [string]$_.DisplayGroup
        group = [string]$_.Group
        enabled = [string]$_.Enabled
        profile = [string]$_.Profile
        platform = @(Convert-Pkg0313StringArray $_.Platform)
        direction = [string]$_.Direction
        action = [string]$_.Action
        edge_traversal_policy = [string]$_.EdgeTraversalPolicy
        loose_source_mapping = [string]$_.LooseSourceMapping
        local_only_mapping = [string]$_.LocalOnlyMapping
        owner = [string]$_.Owner
      }
    }
  )

  function Get-FilterRows {
    param([string]$Command,[string[]]$Properties)
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
      throw "Required firewall filter cmdlet missing: $Command"
    }
    $items = @($rules | & $Command -ErrorAction Stop)
    return @(
      $items |
        ForEach-Object { Convert-Pkg0313FirewallFilter $_ $Properties } |
        Sort-Object @{Expression={ [string]$_.InstanceID }}, @{Expression={ ($_ | ConvertTo-Json -Compress -Depth 8) }}
    )
  }

  return [pscustomobject][ordered]@{
    profiles = $profiles
    rules = $ruleRows
    application_filters = @(Get-FilterRows 'Get-NetFirewallApplicationFilter' @('InstanceID','Program','Package'))
    service_filters = @(Get-FilterRows 'Get-NetFirewallServiceFilter' @('InstanceID','Service'))
    address_filters = @(Get-FilterRows 'Get-NetFirewallAddressFilter' @('InstanceID','LocalAddress','RemoteAddress','LocalUser','RemoteUser','RemoteMachine'))
    port_filters = @(Get-FilterRows 'Get-NetFirewallPortFilter' @('InstanceID','Protocol','LocalPort','RemotePort','IcmpType','DynamicTarget'))
    interface_type_filters = @(Get-FilterRows 'Get-NetFirewallInterfaceTypeFilter' @('InstanceID','InterfaceType'))
    security_filters = @(Get-FilterRows 'Get-NetFirewallSecurityFilter' @('InstanceID','Authentication','Encryption','OverrideBlockRules','LocalUser','RemoteUser','RemoteMachine'))
  }
}

function Get-Pkg0313HostsSnapshot {
  $path = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    return [pscustomobject][ordered]@{ path=$path; exists=$false; size_bytes=0; sha256=$null }
  }
  $item = Get-Item -LiteralPath $path -ErrorAction Stop
  return [pscustomobject][ordered]@{
    path = $path
    exists = $true
    size_bytes = [long]$item.Length
    sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
  }
}

function Get-Pkg0313DnsSnapshot {
  foreach ($required in @('Get-DnsClient','Get-DnsClientServerAddress','Get-DnsClientGlobalSetting')) {
    if (-not (Get-Command $required -ErrorAction SilentlyContinue)) { throw "Required DNS cmdlet missing: $required" }
  }

  $clients = @(
    Get-DnsClient -ErrorAction Stop |
      Sort-Object InterfaceIndex, InterfaceAlias |
      ForEach-Object {
        [pscustomobject][ordered]@{
          interface_index = [int]$_.InterfaceIndex
          interface_alias = [string]$_.InterfaceAlias
          connection_specific_suffix = [string]$_.ConnectionSpecificSuffix
          connection_specific_suffix_search_list = @(Convert-Pkg0313StringArray $_.ConnectionSpecificSuffixSearchList)
          register_this_connections_address = [string]$_.RegisterThisConnectionsAddress
          use_suffix_when_registering = [string]$_.UseSuffixWhenRegistering
        }
      }
  )

  $servers = @(
    Get-DnsClientServerAddress -ErrorAction Stop |
      Sort-Object InterfaceIndex, AddressFamily |
      ForEach-Object {
        [pscustomobject][ordered]@{
          interface_index = [int]$_.InterfaceIndex
          interface_alias = [string]$_.InterfaceAlias
          address_family = [string]$_.AddressFamily
          server_addresses = @(Convert-Pkg0313StringArray $_.ServerAddresses)
        }
      }
  )

  $globalObject = Get-DnsClientGlobalSetting -ErrorAction Stop
  $global = [pscustomobject][ordered]@{
    suffix_search_list = @(Convert-Pkg0313StringArray $globalObject.SuffixSearchList)
    use_devolution = [string]$globalObject.UseDevolution
    devolution_level = [string]$globalObject.DevolutionLevel
  }

  $nrptGlobal = [pscustomobject][ordered]@{ supported=$false; values=$null }
  if (Get-Command Get-DnsClientNrptGlobal -ErrorAction SilentlyContinue) {
    $value = Get-DnsClientNrptGlobal -ErrorAction Stop
    $nrptGlobal = [pscustomobject][ordered]@{
      supported = $true
      values = [pscustomobject][ordered]@{
        enable_da_for_all_networks = [string]$value.EnableDAForAllNetworks
        secure_name_query_fallback = [string]$value.SecureNameQueryFallback
        query_policy = [string]$value.QueryPolicy
      }
    }
  }

  $nrptRules = [pscustomobject][ordered]@{ supported=$false; rules=@() }
  if (Get-Command Get-DnsClientNrptRule -ErrorAction SilentlyContinue) {
    $items = @(
      Get-DnsClientNrptRule -ErrorAction Stop |
        Sort-Object Name, Namespace |
        ForEach-Object {
          [pscustomobject][ordered]@{
            name = [string]$_.Name
            namespace = @(Convert-Pkg0313StringArray $_.Namespace)
            name_servers = @(Convert-Pkg0313StringArray $_.NameServers)
            dnssec_enabled = [string]$_.DnsSecEnabled
            dnssec_ipsec_required = [string]$_.DnsSecIPsecRequired
            dnssec_validation_required = [string]$_.DnsSecValidationRequired
            direct_access_dns_servers = @(Convert-Pkg0313StringArray $_.DirectAccessDnsServers)
            direct_access_enabled = [string]$_.DirectAccessEnabled
            direct_access_proxy_type = [string]$_.DirectAccessProxyType
            direct_access_proxy_name = [string]$_.DirectAccessProxyName
          }
        }
    )
    $nrptRules = [pscustomobject][ordered]@{ supported=$true; rules=$items }
  }

  return [pscustomobject][ordered]@{
    clients = $clients
    server_addresses = $servers
    global = $global
    nrpt_global = $nrptGlobal
    nrpt_rules = $nrptRules
  }
}

function Get-Pkg0313TrustSnapshot {
  $result = @()
  foreach ($location in @('CurrentUser','LocalMachine')) {
    foreach ($store in @('Root','CA','TrustedPublisher','TrustedPeople')) {
      $path = "Cert:\${location}\${store}"
      if (-not (Test-Path -LiteralPath $path)) { throw "Required certificate store unavailable: $path" }
      $thumbprints = @(
        Get-ChildItem -LiteralPath $path -ErrorAction Stop |
          ForEach-Object { ([string]$_.Thumbprint).ToUpperInvariant() } |
          Sort-Object -Unique
      )
      $result += [pscustomobject][ordered]@{
        location = $location
        store = $store
        thumbprints = $thumbprints
      }
    }
  }
  return @($result | Sort-Object location, store)
}

function Get-Pkg0313SystemSnapshot {
  return [pscustomobject][ordered]@{
    schema_version = 1
    firewall = Get-Pkg0313FirewallSnapshot
    hosts = Get-Pkg0313HostsSnapshot
    resolver = Get-Pkg0313DnsSnapshot
    trust = @(Get-Pkg0313TrustSnapshot)
  }
}

function Write-Pkg0313Snapshot {
  param([Parameter(Mandatory=$true)][string]$Path)
  $snapshot = Get-Pkg0313SystemSnapshot
  $json = ($snapshot | ConvertTo-Json -Depth 24 -Compress)
  $utf8 = New-Object System.Text.UTF8Encoding($false)
  [IO.File]::WriteAllText($Path, $json + [Environment]::NewLine, $utf8)
  return [pscustomobject][ordered]@{
    path = $Path
    sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    snapshot = $snapshot
  }
}

function Assert-Pkg0313SnapshotEqual {
  param(
    [Parameter(Mandatory=$true)][string]$BaselinePath,
    [Parameter(Mandatory=$true)][string]$CandidatePath,
    [Parameter(Mandatory=$true)][string]$Label
  )
  $baseline = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $BaselinePath).Path).TrimEnd("`r","`n")
  $candidate = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $CandidatePath).Path).TrimEnd("`r","`n")
  if ($baseline -ne $candidate) {
    throw "03.13 protected Windows state changed during $Label. baseline=$BaselinePath candidate=$CandidatePath"
  }
}
