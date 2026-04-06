#!/usr/bin/env perl
use strict;
use warnings;
use utf8;
use Encode qw(encode);
use Getopt::Long qw(GetOptions);
use POSIX qw(erf);

binmode STDOUT, ':encoding(UTF-8)';

my %opt = (
    kmax        => 8,
    window_bits => 4096,
    demo        => 0,
);

GetOptions(
    'bmp=s'         => \$opt{bmp},
    'avi=s'         => \$opt{avi},
    'wav=s'         => \$opt{wav},
    'txt=s'         => \$opt{txt},
    'sensor-bin=s'  => \$opt{sensor_bin},
    'kmax=i'        => \$opt{kmax},
    'window-bits=i' => \$opt{window_bits},
    'demo!'         => \$opt{demo},
) or die "Invalid options\n";

if ($opt{demo}) {
    create_demo_files();
    $opt{bmp}        //= 'demo.bmp';
    $opt{avi}        //= 'demo.avi';
    $opt{wav}        //= 'demo.wav';
    $opt{txt}        //= 'demo.txt';
    $opt{sensor_bin} //= 'demo_sensor.bin';
}

my @datasets = (
    ['bmp',   $opt{bmp}],
    ['avi',   $opt{avi}],
    ['wav',   $opt{wav}],
    ['txt',   $opt{txt}],
    ['sensor',$opt{sensor_bin}],
);

print "# Binary model verification (Perl)\n";
print "# kmax=$opt{kmax}, window_bits=$opt{window_bits}\n\n";

for my $ds (@datasets) {
    my ($name, $path) = @$ds;
    if (!defined $path || !-f $path) {
        print "[$name] skipped (file not found)\n";
        next;
    }

    my $bits = read_file_bits($path);
    if (@$bits < 64) {
        print "[$name] skipped (too few bits: " . scalar(@$bits) . ")\n";
        next;
    }

    my $stats = compute_basic_stats($bits);
    print_dataset_header($name, $path, $stats, scalar(@$bits));

    my $rows = compute_depth_rows($bits, $stats->{rho}, $opt{kmax}, $opt{window_bits});
    print_depth_table($rows);
    print "\n";
}

sub read_file_bits {
    my ($path) = @_;
    open my $fh, '<:raw', $path or die "open($path): $!";
    local $/;
    my $data = <$fh>;
    close $fh;

    my @bits;
    for my $byte (unpack('C*', $data)) {
        for my $shift (reverse 0..7) {
            push @bits, (($byte >> $shift) & 1);
        }
    }
    return \@bits;
}

sub compute_basic_stats {
    my ($bits) = @_;
    my $n = scalar(@$bits);

    my $ones = 0;
    $ones += $_ for @$bits;
    my $p1 = $ones / $n;

    my ($copy, $pairs) = (0, 0);
    for my $i (1 .. $n - 1) {
        $pairs++;
        $copy++ if $bits->[$i] == $bits->[$i - 1];
    }
    my $p_copy = $pairs > 0 ? $copy / $pairs : 0.5;
    my $rho = 2.0 * $p_copy - 1.0;

    my $hx = h_bin($p1);

    return {
        p1     => $p1,
        p_copy => $p_copy,
        rho    => $rho,
        hx     => $hx,
    };
}

sub compute_depth_rows {
    my ($bits, $rho, $kmax, $window_bits) = @_;
    my $n = scalar(@$bits);
    my $L = $window_bits < $n ? $window_bits : $n;
    $L = 64 if $L < 64;

    my @rows;
    my $prev_delta;
    for my $k (1 .. $kmax) {
        my $Tk = sin(3.141592653589793 / (2 ** $k));
        my $omega = 3.141592653589793 * $k / ($L + 1);
        my $lambda = (1 - $rho * $rho) / (1 - 2 * $rho * cos($omega) + $rho * $rho + 1e-12);
        $lambda = 1e-12 if $lambda <= 0;

        my $m_gauss = erf($Tk / sqrt(2 * $lambda));
        $m_gauss = clamp($m_gauss, 1e-12, 1 - 1e-12);

        my $delta_i = h_bin($m_gauss / 2.0);
        my $r = defined $prev_delta ? ($delta_i / ($prev_delta + 1e-12)) : undef;
        $prev_delta = $delta_i;

        push @rows, {
            k       => $k,
            T       => $Tk,
            lambda  => $lambda,
            m_gauss => $m_gauss,
            delta_i => $delta_i,
            r       => $r,
        };
    }
    return \@rows;
}

sub print_dataset_header {
    my ($name, $path, $stats, $n_bits) = @_;
    printf "[%s] %s\n", $name, $path;
    printf "  bits=%d, p(1)=%.4f, H(X)=%.4f bit, p_copy=%.4f, rho=%.4f\n",
        $n_bits, $stats->{p1}, $stats->{hx}, $stats->{p_copy}, $stats->{rho};
}

sub print_depth_table {
    my ($rows) = @_;
    print "  k |   T_k    | lambda_k | m_gauss  | ΔI_k(bit) | r_k\n";
    print "  --+----------+----------+----------+-----------+--------\n";
    for my $r (@$rows) {
        printf "  %d | %8.5f | %8.5f | %8.5f | %9.5f | %s\n",
            $r->{k}, $r->{T}, $r->{lambda}, $r->{m_gauss}, $r->{delta_i},
            (defined $r->{r} ? sprintf('%.4f', $r->{r}) : '-');
    }
}

sub h_bin {
    my ($p) = @_;
    $p = clamp($p, 1e-12, 1 - 1e-12);
    return -$p * log($p) / log(2) - (1 - $p) * log(1 - $p) / log(2);
}

sub clamp {
    my ($x, $lo, $hi) = @_;
    return $lo if $x < $lo;
    return $hi if $x > $hi;
    return $x;
}

sub create_demo_files {
    write_raw('demo.bmp', "BM" . ("\x00" x 1022));
    write_raw('demo.avi', "RIFF" . ("\x11\x22\x33\x44" x 256));
    write_raw('demo.wav', "RIFF" . ("\x00\x7f\x80\xff" x 256));

    my $txt = "UTF-8テキスト検証: 複素球面玉ねぎ定理とBLTをPerlで確認。\n" x 32;
    write_raw('demo.txt', encode('UTF-8', $txt));

    my @sensor;
    my $seed = 1234567;
    for my $i (0..1023) {
        $seed = (1103515245 * $seed + 12345) & 0x7fffffff;
        my $noise = ($seed / 0x7fffffff) - 0.5;
        my $v = sin($i * 0.04) + 0.3 * sin($i * 0.21) + 0.05 * $noise;
        push @sensor, $v;
    }
    my $packed = pack('f<*', @sensor);
    write_raw('demo_sensor.bin', $packed);
}

sub write_raw {
    my ($path, $data) = @_;
    open my $fh, '>:raw', $path or die "open($path): $!";
    print {$fh} $data;
    close $fh;
}
