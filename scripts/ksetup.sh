#!/usr/bin/bash

declare -a USERS
declare -a KEYTABS
dir="/tmp"

function print_help() {
    echo "This script uses kinit for all users and their respective keytabs. After this is done the script will set the correct permissions on the tgt."
    echo "Every user must be paired with a keytab file. For this the order is used. The first user will be paired with the first keytab, the second user with the second keytab etc."
    echo "Usage: $0 [-h] -g=GROUP -u=USER1 -k=KEYTAB1 -u=USER2 -k=KEYTAB2 ..."
    echo
    echo "arguments: "
    echo "  -d, --dir           The directory that contains the TGT's, set in /etc/krb5.conf. This is /tmp by default"
    echo "  -h, --help          Show this help"
    echo "  -g, --group         The group of for the kerberos files. The omdsite(s) using kerberos must be part of this group"
    echo "  -u, --user          The name of the user. this follows the format: username@REALM"
    echo "  -k, --keytab        The keytabfile of the corresponding user"
    exit 0
}

for arg in "$@"; do
    case $arg in 
        -d|--dir)
            dir="${arg#*=}"
            ;;
        -h|--help)
            print_help
            ;;
        -g=*|--group=*)
            GROUP="${arg#*=}"
            if [ ! $(getent group $GROUP) ]; 
            then
                echo "group $GROUP does not exist"
                exit 1
            fi
            ;;
        -u=*|--user=*)
            USERS+=("${arg#*=}")
            ;;
        -k=*|--keytab=*)
            keytab=${arg#*=}
            if [ -f $keytab ]
            then
                KEYTABS+=($keytab)
            else 
                echo "Keytab $keytab does not exist"
                exit 1
            fi
            ;;
        -*|--*)
            echo "Unknown option: $i"
            exit 1
            ;;
    esac
done

if [ -z $GROUP ]
then
    echo "No group set"
    print_help
fi 

LENGTH=${#USERS[@]}
if [ $LENGTH -eq 0 ]
then 
    echo "No users given"
    print_help
fi

if [ $LENGTH -ne "${#KEYTABS[@]}" ]
then
    echo "Number of users and keytabs are not equal"
    echo 'Be sure to use --user=$user --keytab=$keytab in the correct order, since every user is paired with a keytab'
    print_help()
fi

for idx in $(seq $LENGTH)
do
    usr=${USERS[$idx-1]}
    kt=${KEYTABS[$idx-1]}

    echo "kinit -kt $kt $usr"
    kinit -kt $kt $usr
done

echo
echo "setting permissions: -rw-r----- root $GROUP"

echo "for: $dir/primary"
chown root:$GROUP $dir/primary
chmod o=rw,g=r,o= $dir/primary
while read file
do
    echo "for: $dir/$file"
    chown root:$GROUP $dir/$file
    chmod o=rw,g=r,o= $dir/$file
done < $dir/primary
echo "done"