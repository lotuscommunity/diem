locals {
  forge_helm_chart_path = "${path.module}/../../helm/forge"
}
resource "helm_release" "forge" {
  count       = var.enable_forge ? 1 : 0
  name        = "forge"
  chart       = local.forge_helm_chart_path
  max_history = 2
  wait        = false

  values = [
    jsonencode({
      forge = {
        image = {
          tag = var.image_tag
        }
      }
      serviceAccount = {
        annotations = {
          "eks.amazonaws.com/role-arn" = aws_iam_role.forge.arn
        }
      }
    }),
    jsonencode(var.forge_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.forge_helm_chart_path, "**") : filesha1("${local.forge_helm_chart_path}/${f}")]))
  }
}

data "aws_iam_policy_document" "forge-assume-role" {
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]

    principals {
      type = "Federated"
      identifiers = [
        "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/${module.validator.oidc_provider}"
      ]
    }

    condition {
      test     = "StringEquals"
      variable = "${module.validator.oidc_provider}:sub"
      # the name of the default forge service account
      values = ["system:serviceaccount:default:forge"]
    }

    condition {
      test     = "StringEquals"
      variable = "${module.validator.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "forge" {
  statement {
    sid = "AllowS3"
    actions = [
      "s3:*",
    ]
    resources = [
      "arn:aws:s3:::${var.forge_config_s3_bucket}*",
      "arn:aws:s3:::${var.forge_config_s3_bucket}/*"
    ]
  }
}

resource "aws_iam_role" "forge" {
  name                 = "diem-node-testnet-${local.workspace_name}-forge"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.forge-assume-role.json
}

resource "aws_iam_role_policy" "forge" {
  name   = "Helm"
  role   = aws_iam_role.forge.name
  policy = data.aws_iam_policy_document.forge.json
}

