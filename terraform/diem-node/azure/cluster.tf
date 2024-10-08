locals {
  kubernetes_version = "1.22.6"
}

resource "azurerm_kubernetes_cluster" "diem" {
  name                            = "diem-${terraform.workspace}"
  resource_group_name             = azurerm_resource_group.diem.name
  location                        = azurerm_resource_group.diem.location
  dns_prefix                      = "diem-${terraform.workspace}"
  kubernetes_version              = local.kubernetes_version
  api_server_authorized_ip_ranges = concat(["${azurerm_public_ip.nat.ip_address}/32"], var.k8s_api_sources)

  network_profile {
    network_plugin    = "kubenet"
    network_policy    = "calico"
    load_balancer_sku = "standard"
  }

  default_node_pool {
    name                 = "utilities"
    orchestrator_version = local.kubernetes_version
    vm_size              = var.utility_instance_type
    vnet_subnet_id       = azurerm_subnet.nodes.id
    node_count           = lookup(var.node_pool_sizes, "utilities", var.utility_instance_num)
    os_disk_size_gb      = 30
  }

  service_principal {
    client_id     = azuread_service_principal.diem.application_id
    client_secret = azuread_application_password.diem.value
  }
}

resource "azurerm_kubernetes_cluster_node_pool" "validators" {
  kubernetes_cluster_id = azurerm_kubernetes_cluster.diem.id
  orchestrator_version  = azurerm_kubernetes_cluster.diem.kubernetes_version

  name            = "validators"
  vm_size         = var.validator_instance_type
  vnet_subnet_id  = azurerm_subnet.nodes.id
  node_count      = lookup(var.node_pool_sizes, "validators", var.validator_instance_num)
  os_disk_size_gb = 30
  node_taints     = var.validator_instance_enable_taint ? ["diem.org/nodepool=validators:NoExecute"] : []
}

resource "azurerm_log_analytics_workspace" "diem" {
  name                = "diem-${terraform.workspace}"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
  retention_in_days   = 30
}

resource "azurerm_monitor_diagnostic_setting" "cluster" {
  name                       = "cluster"
  target_resource_id         = azurerm_kubernetes_cluster.diem.id
  log_analytics_workspace_id = azurerm_log_analytics_workspace.diem.id

  log { category = "kube-apiserver" }
  log { category = "kube-controller-manager" }
  log { category = "kube-scheduler" }
  log { category = "kube-audit" }
  log { category = "guard" }
}
