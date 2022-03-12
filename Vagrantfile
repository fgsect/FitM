# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
    # For a complete reference, please see the online documentation at
    # https://docs.vagrantup.com.
    config.vm.box = "ubuntu/focal64"
  
    config.vm.network "forwarded_port", guest: 22, host: 2223, host_ip: "127.0.0.1"
    
     config.vm.provider "virtualbox" do |vb|
       vb.memory = "4096"
       vb.name = "FitM"
     end
     config.vm.define :vm do |vm_settings|

      # Small hack for development, mounting the apt cache to a local tmp folder
     # to accell the process of fetching mysql packges.
     vm_settings.vm.synced_folder "./tmp", "/var/cache/apt/archives/"
    end
  
    config.vm.provision "shell" do |s|
      s.path = "provision.sh"
    end
  
  end